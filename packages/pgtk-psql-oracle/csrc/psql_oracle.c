/*-------------------------------------------------------------------------
 *
 * psql_oracle.c
 *	  An oracle for psql's own client-side lexers.
 *
 * This shim drives `src/fe_utils/psqlscan.l` and `src/bin/psql/psqlscanslash.l`
 * of the pinned PostgreSQL release, and reports the facts that pgtk-psql
 * models: the text psql forwards to the server, where each variable
 * interpolation is, where a query buffer is submitted, and where each
 * backslash command and each of its arguments begins and ends.
 *
 * The shim reads the arguments of each command as `HandleSlashCmds` and
 * `exec_command` in `src/bin/psql/command.c` read them: with the same
 * `psql_scan_slash_option` calls and the same option types.  It runs no
 * command.  It applies only `\set`, `\unset`, `\if`, `\elif`, `\else` and
 * `\endif`, because those change what psql sends.  It never runs a
 * backquoted shell command: the lexer's passthrough is a conditional stack
 * whose only entry is inactive, so psqlscanslash.l does not evaluate one.
 *
 * When psql sends a query buffer that holds `COPY ... FROM STDIN`, or runs
 * `\copy ... from stdin`, it reads the data lines from the script itself
 * (`handleCopyIn` in `src/bin/psql/copy.c`), from the line after the line
 * that sent the buffer up to a line that is exactly `\.`.  The shim does the
 * same: it reports each data block and does not lex it.  It then lexes the
 * rest of the line that sent the buffer, and continues after the data, as
 * psql does.  It counts the COPY commands as `MainLoop` and
 * `ExecQueryAndProcessResults` count them, with `psql_scan_count_copy_from_stdin`.
 *
 * Two adaptations of psql's own main loop are deliberate, and both follow
 * pgtk-psql's document model (ADR 0007):
 *
 * 1. The SQL text of the document is scanned as one input string, not one
 *	  line at a time.  The flex rules of psqlscan.l do not read line
 *	  boundaries, so the lexing is the same.  The arguments of a backslash
 *	  command are read from the rest of its line only, which is where psql's
 *	  own input buffer ends; SQL lexing then continues where the command
 *	  ends.
 *
 * 2. The query buffer is never reset.  psql resets it after every
 *	  submission, and the `{whitespace}` rule of psqlscan.l does not echo
 *	  while the buffer is empty, so psql drops the leading trivia of every
 *	  statement.  pgtk-psql renders one buffer for a whole document, so the
 *	  buffer is seeded with one byte that is removed at the end; the trivia
 *	  then survives and the two texts are comparable.  The lexer state is
 *	  reset where psql resets it (`psql_scan_reset`), so the parenthesis and
 *	  `BEGIN` depths start again after each submission.
 *
 *-------------------------------------------------------------------------
 */
#include "postgres_fe.h"

#include <stdlib.h>
#include <string.h>

#include "fe_utils/conditional.h"
#include "fe_utils/psqlscan.h"
#include "fe_utils/psqlscan_int.h"
#include "psqlscanslash.h"
#include "pqexpbuffer.h"
#include "stringutils.h"
#include "mb/pg_wchar.h"

/*
 * Accessors that flex generates for psqlscan.l (`%option prefix="psql_yy"`).
 * `yytext` points into `state->scanbuf`, which psqlscan_prepare_buffer hands
 * straight to `yy_scan_buffer`, so the difference of the two pointers is a
 * byte offset into the input of the current psql_scan_setup.  psqlscanslash.l
 * runs on the same scanner object, so the same accessors also give its
 * position.
 */
extern char *psql_yyget_text(yyscan_t yyscanner);
extern size_t psql_yyget_leng(yyscan_t yyscanner);

/*
 * psqlscanslash.l calls this after it has evaluated a backquoted argument.
 * This oracle never evaluates one (see the file header), so the definition
 * only satisfies the linker; psql's own is in src/bin/psql/common.c.
 */
void		SetShellResultVariables(int exit_code);

void
SetShellResultVariables(int exit_code)
{
	(void) exit_code;
}

/* Event kinds; kept in step with the Rust `read_event`. */
#define PGPSO_INTERPOLATION 1
#define PGPSO_SEMICOLON 2
#define PGPSO_BACKSLASH 3
#define PGPSO_EOL 4
#define PGPSO_INCOMPLETE 5
#define PGPSO_OPTION 6
#define PGPSO_DISCARD 7
#define PGPSO_COPY_DATA 8

typedef struct PgpsoEvent
{
	int			kind;
	/* Extent in the document, or -1 when the token came from an expansion. */
	int			source_start;
	int			source_end;
	/* Extent in the forwarded text.  `output_end` is -1 when psql rescans
	 * the substituted value, which only a `:name` substitution does. */
	int			output_start;
	int			output_end;
	/* PsqlScanQuoteType for an interpolation, promptStatus_t for a stop,
	 * the name length for a backslash command, and 1 for an option whose
	 * value psql could not read (an unterminated quote). */
	int			detail;
	/* 1 when the variable was bound, 0 when it was not. */
	int			bound;
	/* Variable name, command name or option value; malloc'd, or NULL. */
	char	   *text;
} PgpsoEvent;

typedef struct PgpsoResult
{
	PgpsoEvent *events;
	int			nevents;
	int			capacity;
	/* The text psql forwards to the server, for the whole document. */
	char	   *sql;
	int			sql_len;
} PgpsoResult;

typedef struct PgpsoContext
{
	/* The psql variables: the bindings of the caller, then \set and \unset. */
	char	  **names;
	char	  **values;
	int			nvars;
	int			capacity;
	PsqlScanState state;
	PQExpBuffer output;
	PgpsoResult *result;
	/* Document offset of the current psql_scan_setup. */
	int			base;
	/* Bytes of seed at the start of `output`. */
	int			seed;
	/* The \if stack of the script, as MainLoop keeps it. */
	ConditionalStack cstack;
	/* The document. */
	const char *source;
	int			source_len;
	/* Where the current query buffer starts in `output`. */
	int			buffer_start;
	/* Where the next block of COPY data starts, or -1. */
	int			copy_pos;
	/*
	 * The end of the line whose sends read COPY data, or -1.  The lexer reads
	 * to here, and then goes on at `copy_pos`.
	 */
	int			copy_line_end;
} PgpsoContext;

/*
 * The context of the scan in progress.  psql's lexer keeps its state in
 * file-scope tables, so the Rust side serialises every call, and one context
 * at a time is enough.  The passthrough pointer of the lexer is the inactive
 * conditional stack (see the file header), so the callback reads this.
 */
static PgpsoContext *current_context;

/* `space` of psqlscanslash.l, which holds the vertical tab from 17. */
#if PG_VERSION_NUM >= 170000
#define SLASH_SPACE " \t\n\r\f\v"
#else
#define SLASH_SPACE " \t\n\r\f"
#endif

static PgpsoEvent *
push_event(PgpsoResult *result, int kind)
{
	PgpsoEvent *event;

	if (result->nevents == result->capacity)
	{
		result->capacity = result->capacity ? result->capacity * 2 : 16;
		result->events = realloc(result->events,
								 (size_t) result->capacity * sizeof(PgpsoEvent));
	}
	event = &result->events[result->nevents++];
	memset(event, 0, sizeof(*event));
	event->kind = kind;
	event->source_start = -1;
	event->source_end = -1;
	event->output_start = -1;
	event->output_end = -1;
	event->detail = -1;
	return event;
}

/*
 * The extent of the token flex has just matched, in document offsets.
 * Returns 0 when the token is not in the outer buffer, which happens while a
 * `:name` substitution is being rescanned.
 */
static int
current_token(PgpsoContext *context, int *start, int *end)
{
	const char *text;

	if (context->state->buffer_stack != NULL)
		return 0;
	text = psql_yyget_text(context->state->scanner);
	*start = context->base + (int) (text - context->state->scanbuf);
	*end = *start + (int) psql_yyget_leng(context->state->scanner);
	return 1;
}

/*
 * The document offset where the lexer stands after a psqlscanslash.l call.
 *
 * Every rule of psqlscanslash.l that returns either ends its match there or
 * gives the stop character back with yyless(0), so the end of the last match
 * is the position.  At end of input flex leaves yytext at the end of the
 * buffer, so the result is limited to `limit`, the end of the current input.
 */
static int
slash_position(PgpsoContext *context, int limit)
{
	const char *text = psql_yyget_text(context->state->scanner);
	int			at = context->base + (int) (text - context->state->scanbuf) +
		(int) psql_yyget_leng(context->state->scanner);

	return at < limit ? at : limit;
}

static int
find_variable(PgpsoContext *context, const char *name)
{
	int			i;

	for (i = 0; i < context->nvars; i++)
		if (strcmp(context->names[i], name) == 0)
			return i;
	return -1;
}

/*
 * SetVariable without the hooks of psql's special variables: `value` NULL
 * unsets.  A name that valid_variable_name refuses changes nothing.
 */
static void
set_variable(PgpsoContext *context, const char *name, const char *value)
{
	int			i;
	const unsigned char *ptr = (const unsigned char *) name;

	if (*ptr == '\0')
		return;
	for (; *ptr; ptr++)
		if (!IS_HIGHBIT_SET(*ptr) &&
			strchr("ABCDEFGHIJKLMNOPQRSTUVWXYZ" "abcdefghijklmnopqrstuvwxyz"
				   "_0123456789", *ptr) == NULL)
			return;

	i = find_variable(context, name);
	if (value == NULL)
	{
		if (i >= 0)
		{
			free(context->names[i]);
			free(context->values[i]);
			context->names[i] = context->names[context->nvars - 1];
			context->values[i] = context->values[context->nvars - 1];
			context->nvars--;
		}
		return;
	}
	if (i >= 0)
	{
		free(context->values[i]);
		context->values[i] = strdup(value);
		return;
	}
	if (context->nvars == context->capacity)
	{
		context->capacity = context->capacity ? context->capacity * 2 : 16;
		context->names = realloc(context->names,
								 (size_t) context->capacity * sizeof(char *));
		context->values = realloc(context->values,
								  (size_t) context->capacity * sizeof(char *));
	}
	context->names[context->nvars] = strdup(name);
	context->values[context->nvars] = strdup(value);
	context->nvars++;
}

/*
 * Quote `value` as a SQL string literal, as libpq's PQescapeLiteral does
 * (`src/interfaces/libpq/fe-exec.c`, PQescapeInternal): every `'` and every
 * `\` is doubled, and a value holding a backslash is written as ` E'...'` so
 * that it means the same under either standard_conforming_strings.
 *
 * psql itself calls PQescapeLiteral, which needs the client encoding and the
 * standard_conforming_strings of a live connection.  This oracle has no
 * connection, so it applies the same algorithm for the UTF-8, standard-strings
 * case that pgtk-psql assumes.
 */
static char *
escape_literal(const char *value)
{
	size_t		length = strlen(value);
	size_t		specials = 0;
	size_t		backslashes = 0;
	char	   *out;
	char	   *write;
	size_t		i;

	for (i = 0; i < length; i++)
	{
		if (value[i] == '\'')
			specials++;
		else if (value[i] == '\\')
		{
			specials++;
			backslashes++;
		}
	}

	out = malloc(length + specials + 5);
	write = out;
	if (backslashes > 0)
	{
		*write++ = ' ';
		*write++ = 'E';
	}
	*write++ = '\'';
	for (i = 0; i < length; i++)
	{
		if (value[i] == '\'' || value[i] == '\\')
			*write++ = value[i];
		*write++ = value[i];
	}
	*write++ = '\'';
	*write = '\0';
	return out;
}

/* Quote `value` as a quoted identifier, as PQescapeIdentifier does. */
static char *
escape_identifier(const char *value)
{
	size_t		length = strlen(value);
	size_t		quotes = 0;
	char	   *out;
	char	   *write;
	size_t		i;

	for (i = 0; i < length; i++)
		if (value[i] == '"')
			quotes++;

	out = malloc(length + quotes + 3);
	write = out;
	*write++ = '"';
	for (i = 0; i < length; i++)
	{
		if (value[i] == '"')
			*write++ = '"';
		*write++ = value[i];
	}
	*write++ = '"';
	*write = '\0';
	return out;
}


/*
 * The lexer's `get_variable` callback.  It is the one place psqlscan.l and
 * psqlscanslash.l tell a client that they have recognised an interpolation,
 * so every interpolation event is recorded here rather than inferred from
 * the text.  As psql_get_variable in src/bin/psql/common.c does, it
 * substitutes nothing in an inactive \if branch.
 */
static char *
pgpso_get_variable(const char *varname, PsqlScanQuoteType quote,
				   void *passthrough)
{
	PgpsoContext *context = current_context;
	PgpsoEvent *event;
	const char *value = NULL;
	char	   *result = NULL;
	int			start;
	int			end;
	int			i;

	(void) passthrough;
	i = find_variable(context, varname);
	if (i >= 0 && conditional_active(context->cstack))
		value = context->values[i];

	event = push_event(context->result, PGPSO_INTERPOLATION);
	event->detail = (int) quote;
	event->bound = value != NULL;
	event->text = strdup(varname);
	event->output_start = (int) context->output->len - context->seed;
	if (current_token(context, &start, &end))
	{
		event->source_start = start;
		event->source_end = end;
	}

	if (value == NULL)
		return NULL;

	switch (quote)
	{
		case PQUOTE_PLAIN:
			result = strdup(value);
			break;
		case PQUOTE_SQL_LITERAL:
			result = escape_literal(value);
			break;
		case PQUOTE_SQL_IDENT:
			result = escape_identifier(value);
			break;
		case PQUOTE_SHELL_ARG:
			/* Only a backquoted argument asks for this, and this oracle
			 * never runs one. */
			result = strdup(value);
			break;
	}

	/*
	 * A quoted form is emitted in place, so its extent in the forwarded text
	 * is known now.  A `:name` value is pushed back into the lexer and
	 * rescanned, so its extent is not.  An argument value goes to the
	 * option buffer, not to the forwarded text.
	 */
	if (quote != PQUOTE_PLAIN)
		event->output_end = event->output_start + (int) strlen(result);
	return result;
}

/* How exec_command reads the arguments of a command. */
typedef enum OptionShape
{
	SHAPE_NORMAL,				/* OT_NORMAL options */
	SHAPE_WHOLE_LINE,			/* one OT_WHOLE_LINE option */
	SHAPE_FILEPIPE,				/* one OT_FILEPIPE option */
	SHAPE_G,					/* \g and \gx: (options), then OT_FILEPIPE */
	SHAPE_UNKNOWN				/* not a command: psql discards the line */
} OptionShape;

static int
is_one_of(const char *name, const char *const *names)
{
	for (; *names; names++)
		if (strcmp(name, *names) == 0)
			return 1;
	return 0;
}

/*
 * The shape of `name`, from exec_command in src/bin/psql/command.c of the
 * pinned release.  Every name that starts with `d` goes to exec_command_d
 * and every name that starts with `lo_` to exec_command_lo, so both are
 * read as OT_NORMAL here.
 */
static OptionShape
shape_of(const char *name)
{
	static const char *const whole_line[] = {
		"ef", "ev", "h", "help", "sf", "sf+", "sv", "sv+", "unrestrict", "!", NULL
	};
	static const char *const filepipe[] = {"o", "out", "w", "write", NULL};
	static const char *const normal[] = {
		"a", "C", "c", "connect", "cd", "conninfo", "copyright", "crosstabview",
		"e", "edit", "echo", "qecho", "warn", "elif", "else", "endif", "encoding",
		"errverbose", "f", "gdesc", "gexec", "gset", "H", "html", "i", "include",
		"ir", "include_relative", "if", "l", "list", "l+", "list+", "p", "print",
		"password", "prompt", "pset", "q", "quit", "r", "reset", "restrict", "s",
		"set", "setenv", "t", "T", "timing", "unset", "watch", "x", "z", "?",
#if PG_VERSION_NUM >= 150000
		"getenv",
#endif
#if PG_VERSION_NUM >= 160000
		"bind", "zS",
#endif
#if PG_VERSION_NUM >= 180000
		"bind_named", "close_prepared", "endpipeline", "flush", "flushrequest",
		"getresults", "lx", "listx", "lx+", "listx+", "l+x", "list+x", "parse",
		"sendpipeline", "startpipeline", "syncpipeline", "zx", "zSx", "zxS",
#endif
		NULL
	};

	if (pg_strcasecmp(name, "copy") == 0 || is_one_of(name, whole_line))
		return SHAPE_WHOLE_LINE;
	if (is_one_of(name, filepipe))
		return SHAPE_FILEPIPE;
	if (strcmp(name, "g") == 0 || strcmp(name, "gx") == 0)
		return SHAPE_G;
	if (name[0] == 'd' || strncmp(name, "lo_", 3) == 0 || is_one_of(name, normal))
		return SHAPE_NORMAL;
	return SHAPE_UNKNOWN;
}

/*
 * One psql_scan_slash_option call, recorded as an option event.  Returns the
 * value, which the caller frees, or NULL when there is no more option.
 * `limit` is the end of the line.
 */
static char *
read_option(PgpsoContext *context, enum slash_option_type type, int limit)
{
	const char *source = context->state->scanline;
	int			before = slash_position(context, limit);
	int			start;
	int			end;
	char		quote = 0;
	char	   *value;
	PgpsoEvent *event;

	value = psql_scan_slash_option(context->state, type, &quote, false);
	end = slash_position(context, limit);
	/* xslashargstart eats the space before the option. */
	start = before;
	while (start < end &&
		   strchr(SLASH_SPACE, source[start - context->base]) != NULL &&
		   source[start - context->base] != '\0')
		start++;
	if (value == NULL && start >= end)
		return NULL;

	event = push_event(context->result, PGPSO_OPTION);
	event->source_start = start;
	event->source_end = end;
	event->detail = value == NULL;
	event->text = value ? strdup(value) : NULL;
	return value;
}

/* Reads options of `type` until there are no more. */
static void
read_all_options(PgpsoContext *context, enum slash_option_type type, int limit)
{
	char	   *value;

	while ((value = read_option(context, type, limit)) != NULL)
		free(value);
}

/* ParseVariableBool of src/bin/psql/variables.c, without the message. */
static int
parse_bool(const char *value, int *result)
{
	size_t		len = strlen(value);

	if (len > 0 && pg_strncasecmp(value, "true", len) == 0)
		*result = 1;
	else if (len > 0 && pg_strncasecmp(value, "false", len) == 0)
		*result = 0;
	else if (len > 0 && pg_strncasecmp(value, "yes", len) == 0)
		*result = 1;
	else if (len > 0 && pg_strncasecmp(value, "no", len) == 0)
		*result = 0;
	else if (pg_strncasecmp(value, "on", (len > 2 ? len : 2)) == 0)
		*result = 1;
	else if (pg_strncasecmp(value, "off", (len > 2 ? len : 2)) == 0)
		*result = 0;
	else if (pg_strcasecmp(value, "1") == 0)
		*result = 1;
	else if (pg_strcasecmp(value, "0") == 0)
		*result = 0;
	else
		return 0;
	return 1;
}

/* is_true_boolean_expression and gather_boolean_expression of command.c. */
static int
read_expression(PgpsoContext *context, int limit)
{
	PQExpBufferData expression;
	char	   *value;
	int			count = 0;
	int			result = 0;
	int			valid;

	initPQExpBuffer(&expression);
	while ((value = read_option(context, OT_NORMAL, limit)) != NULL)
	{
		if (count++ > 0)
			appendPQExpBufferChar(&expression, ' ');
		appendPQExpBufferStr(&expression, value);
		free(value);
	}
	valid = parse_bool(expression.data, &result);
	termPQExpBuffer(&expression);
	return valid && result;
}

/*
 * discard_query_text of command.c: psql takes the query text of the branch
 * that ends out of the buffer.  This shim keeps the text, so that the
 * offsets of earlier events stay valid, and reports the range that psql
 * removes; the Rust side removes it.  The buffer is never empty (adaptation
 * 2), so the lexer does not see a difference.
 */
static void
discard_query_text(PgpsoContext *context)
{
	int			length = conditional_stack_get_query_len(context->cstack);
	PgpsoEvent *event;

	event = push_event(context->result, PGPSO_DISCARD);
	event->output_start = (length >= context->seed ? length : context->seed) - context->seed;
	event->output_end = (int) context->output->len - context->seed;
	/* The lexer state, such as the parenthesis depth, goes back too. */
	psql_scan_set_lex_state(context->state,
							conditional_stack_get_lex_state(context->cstack));
}

static void
save_query_text_state(PgpsoContext *context)
{
	conditional_stack_set_query_len(context->cstack, (int) context->output->len);
	conditional_stack_set_lex_state(context->cstack,
									psql_scan_get_lex_state(context->state));
}

/*
 * exec_command_if, _elif, _else and _endif of command.c.  Returns 0 when psql
 * refuses the command, which then discards the rest of the line.
 */
static int
run_conditional(PgpsoContext *context, const char *name, int limit)
{
	ConditionalStack cstack = context->cstack;

	if (strcmp(name, "if") == 0)
	{
		if (conditional_active(cstack))
		{
			conditional_stack_push(cstack, IFSTATE_TRUE);
			save_query_text_state(context);
			if (!read_expression(context, limit))
				conditional_stack_poke(cstack, IFSTATE_FALSE);
		}
		else
		{
			conditional_stack_push(cstack, IFSTATE_IGNORED);
			save_query_text_state(context);
			read_expression(context, limit);
		}
		return 1;
	}
	if (strcmp(name, "elif") == 0)
	{
		switch (conditional_stack_peek(cstack))
		{
			case IFSTATE_TRUE:
				save_query_text_state(context);
				conditional_stack_poke(cstack, IFSTATE_IGNORED);
				read_expression(context, limit);
				return 1;
			case IFSTATE_FALSE:
				discard_query_text(context);
				conditional_stack_poke(cstack, IFSTATE_TRUE);
				if (!read_expression(context, limit))
					conditional_stack_poke(cstack, IFSTATE_FALSE);
				return 1;
			case IFSTATE_IGNORED:
				discard_query_text(context);
				read_expression(context, limit);
				return 1;
			default:
				return 0;
		}
	}
	if (strcmp(name, "else") == 0)
	{
		switch (conditional_stack_peek(cstack))
		{
			case IFSTATE_TRUE:
				save_query_text_state(context);
				conditional_stack_poke(cstack, IFSTATE_ELSE_FALSE);
				return 1;
			case IFSTATE_FALSE:
				discard_query_text(context);
				conditional_stack_poke(cstack, IFSTATE_ELSE_TRUE);
				return 1;
			case IFSTATE_IGNORED:
				discard_query_text(context);
				conditional_stack_poke(cstack, IFSTATE_ELSE_FALSE);
				return 1;
			default:
				return 0;
		}
	}
	/* endif */
	switch (conditional_stack_peek(cstack))
	{
		case IFSTATE_TRUE:
		case IFSTATE_ELSE_TRUE:
			conditional_stack_pop(cstack);
			return 1;
		case IFSTATE_FALSE:
		case IFSTATE_IGNORED:
		case IFSTATE_ELSE_FALSE:
			discard_query_text(context);
			conditional_stack_pop(cstack);
			return 1;
		default:
			return 0;
	}
}

/* What a backslash command does with the query buffer. */
typedef enum BufferEffect
{
	BUFFER_KEEP,				/* the buffer goes on */
	BUFFER_SEND,				/* PSQL_CMD_SEND, and psql reads COPY data */
	BUFFER_SEND_NO_COPY,		/* PSQL_CMD_SEND, but no COPY data is read */
	BUFFER_DISCARD				/* the buffer ends and is not sent */
} BufferEffect;

/*
 * The effect of `name`, as the Rust `classify` gives it.  `\gdesc` runs
 * DescribeQuery, which reads no COPY data, and from 18 `\sendpipeline`
 * queues the query in a pipeline, where ExecQueryAndProcessResults returns
 * before it counts the COPY commands.  `\watch` runs the query again and
 * again; the shim reads its data once.
 */
static BufferEffect
buffer_effect(const char *name)
{
	static const char *const send[] = {
		"crosstabview", "g", "gx", "gexec", "gset", "watch",
#if PG_VERSION_NUM >= 180000
		"parse",
#endif
		NULL
	};
	static const char *const send_no_copy[] = {
		"gdesc",
#if PG_VERSION_NUM >= 180000
		"sendpipeline",
#endif
		NULL
	};
	static const char *const discard[] = {
		"r", "reset",
#if PG_VERSION_NUM >= 180000
		"close_prepared", "endpipeline", "flush", "flushrequest",
		"getresults", "startpipeline", "syncpipeline",
#endif
		NULL
	};

	if (is_one_of(name, send))
		return BUFFER_SEND;
	if (is_one_of(name, send_no_copy))
		return BUFFER_SEND_NO_COPY;
	if (is_one_of(name, discard))
		return BUFFER_DISCARD;
	return BUFFER_KEEP;
}

/*
 * The decision of parse_slash_copy (src/bin/psql/copy.c): whether the
 * arguments of `\copy` say `from stdin` (or `from stdout`), so that do_copy
 * reads the data from the script, `pset.cur_cmd_source`.  The tokens come
 * from psql's own strtokx; the shim assumes standard_conforming_strings.
 */
static int
copy_reads_script(const char *args)
{
	const char *whitespace = " \t\n\r";
	char	   *token;

	if (args == NULL)
		return 0;
	token = strtokx(args, whitespace, ".,()", "\"", 0, false, false, PG_UTF8);
	if (!token)
		return 0;
	if (pg_strcasecmp(token, "binary") == 0)
	{
		token = strtokx(NULL, whitespace, ".,()", "\"", 0, false, false, PG_UTF8);
		if (!token)
			return 0;
	}
	if (token[0] == '(')
	{
		int			parens = 1;

		while (parens > 0)
		{
			token = strtokx(NULL, whitespace, "()", "\"'", 0, true, false, PG_UTF8);
			if (!token)
				return 0;
			if (token[0] == '(')
				parens++;
			else if (token[0] == ')')
				parens--;
		}
	}
	token = strtokx(NULL, whitespace, ".,()", "\"", 0, false, false, PG_UTF8);
	if (!token)
		return 0;
	if (token[0] == '.')
	{
		token = strtokx(NULL, whitespace, ".,()", "\"", 0, false, false, PG_UTF8);
		if (!token)
			return 0;
		token = strtokx(NULL, whitespace, ".,()", "\"", 0, false, false, PG_UTF8);
		if (!token)
			return 0;
	}
	if (token[0] == '(')
	{
		for (;;)
		{
			token = strtokx(NULL, whitespace, "()", "\"", 0, false, false, PG_UTF8);
			if (!token)
				return 0;
			if (token[0] == ')')
				break;
		}
		token = strtokx(NULL, whitespace, ".,()", "\"", 0, false, false, PG_UTF8);
		if (!token)
			return 0;
	}
	if (pg_strcasecmp(token, "from") != 0)
		return 0;
	token = strtokx(NULL, whitespace, ";", "'", 0, false, false, PG_UTF8);
	if (!token)
		return 0;
	return pg_strcasecmp(token, "stdin") == 0 || pg_strcasecmp(token, "stdout") == 0;
}

/*
 * The number of COPY commands of the current query buffer, as
 * ExecQueryAndProcessResults counts them when a send command gives it -1: a
 * new lexer reads the buffer text with one psql_scan call, which stops at the
 * first `;`, and psql_scan_count_copy_from_stdin counts.  The buffer text is
 * the output from `buffer_start`, without the ranges that psql discards.
 */
static int
count_copy_of_buffer(PgpsoContext *context)
{
	PQExpBufferData text;
	PQExpBufferData scratch;
	PsqlScanCallbacks callbacks;
	PsqlScanState state;
	promptStatus_t prompt;
	int			length = (int) context->output->len - context->seed;
	char	   *keep = malloc((size_t) length + 1);
	int			count;
	int			i;

	memset(keep, 1, (size_t) length + 1);
	for (i = 0; i < context->result->nevents; i++)
	{
		PgpsoEvent *event = &context->result->events[i];
		int			at;

		if (event->kind != PGPSO_DISCARD)
			continue;
		for (at = event->output_start; at < event->output_end && at < length; at++)
			keep[at] = 0;
	}
	initPQExpBuffer(&text);
	for (i = context->buffer_start; i < length; i++)
		if (keep[i])
			appendPQExpBufferChar(&text, context->output->data[context->seed + i]);
	free(keep);

	/* No variable callback: the text is already substituted. */
	memset(&callbacks, 0, sizeof(callbacks));
	state = psql_scan_create(&callbacks);
	psql_scan_setup(state, text.data, (int) text.len, PG_UTF8, true);
	initPQExpBuffer(&scratch);
	(void) psql_scan(state, &scratch, &prompt);
	count = psql_scan_count_copy_from_stdin(state);
	psql_scan_finish(state);
	psql_scan_destroy(state);
	termPQExpBuffer(&scratch);
	termPQExpBuffer(&text);
	return count;
}

/*
 * psql reads `count` blocks of COPY data after a send that ends at the
 * document offset `end`.  handleCopyIn reads whole lines from the line after
 * the current one, up to a line that is exactly `\.` and LF or CRLF, or to
 * the end of input.  The shim reports each block, and lexes the rest of the
 * current line before it goes on after the data.
 */
static void
read_copy_data(PgpsoContext *context, int end, int count)
{
	const char *source = context->source;
	int			line_end = end;

	while (line_end < context->source_len && source[line_end] != '\n')
		line_end++;
	if (count <= 0 || line_end >= context->source_len)
		return;
	if (context->copy_pos < line_end + 1)
		context->copy_pos = line_end + 1;
	context->copy_line_end = line_end + 1;

	for (; count > 0 && context->copy_pos < context->source_len; count--)
	{
		int			rows = context->copy_pos;
		int			line = rows;
		PgpsoEvent *event = push_event(context->result, PGPSO_COPY_DATA);

		event->source_start = rows;
		while (line < context->source_len)
		{
			int			next = line;
			int			length;

			while (next < context->source_len && source[next] != '\n')
				next++;
			if (next < context->source_len)
				next++;
			length = next - line;
			if ((length == 3 && memcmp(source + line, "\\.\n", 3) == 0) ||
				(length == 4 && memcmp(source + line, "\\.\r\n", 4) == 0))
			{
				event->output_start = line;
				event->output_end = next;
				break;
			}
			line = next;
		}
		event->source_end = line;
		context->copy_pos = event->output_end >= 0 ? event->output_end : line;
	}
}

/*
 * Reads the arguments of the command `name` as HandleSlashCmds and
 * exec_command read them, and applies \set, \unset and the conditional
 * commands.  `limit` is the end of the line.  Returns the number of COPY
 * data blocks that `\copy` reads.
 */
static int
read_command(PgpsoContext *context, const char *name, int limit)
{
	int			success = 1;
	int			copies = 0;
	char	   *value;

	if (strcmp(name, "if") == 0 || strcmp(name, "elif") == 0 ||
		strcmp(name, "else") == 0 || strcmp(name, "endif") == 0)
		success = run_conditional(context, name, limit);
	else if (strcmp(name, "set") == 0 && conditional_active(context->cstack))
	{
		char	   *variable = read_option(context, OT_NORMAL, limit);

		if (variable != NULL)
		{
			PQExpBufferData assigned;

			initPQExpBuffer(&assigned);
			while ((value = read_option(context, OT_NORMAL, limit)) != NULL)
			{
				appendPQExpBufferStr(&assigned, value);
				free(value);
			}
			set_variable(context, variable, assigned.data);
			termPQExpBuffer(&assigned);
			free(variable);
		}
	}
	else if (strcmp(name, "unset") == 0 && conditional_active(context->cstack))
	{
		value = read_option(context, OT_NORMAL, limit);
		if (value != NULL)
		{
			set_variable(context, value, NULL);
			free(value);
		}
		else
			success = 0;
	}
	else
	{
		switch (shape_of(name))
		{
			case SHAPE_WHOLE_LINE:
				value = read_option(context, OT_WHOLE_LINE, limit);
				/* exec_command_copy runs only in an active branch. */
				if (pg_strcasecmp(name, "copy") == 0 &&
					conditional_active(context->cstack) &&
					copy_reads_script(value))
					copies = 1;
				free(value);
				break;
			case SHAPE_FILEPIPE:
				value = read_option(context, OT_FILEPIPE, limit);
				free(value);
				break;
			case SHAPE_G:
				value = read_option(context, OT_FILEPIPE, limit);
				if (value != NULL && value[0] == '(')
				{
					/* process_command_g_options: through the trailing ')' */
					size_t		length = strlen(value);

					while (!(length > 0 && value[length - 1] == ')'))
					{
						free(value);
						value = read_option(context, OT_NORMAL, limit);
						if (value == NULL)
							break;
						length = strlen(value);
					}
					free(value);
					value = read_option(context, OT_FILEPIPE, limit);
				}
				free(value);
				break;
			case SHAPE_NORMAL:
				break;
			case SHAPE_UNKNOWN:
				success = 0;
				break;
		}
	}

	if (success)
		read_all_options(context, OT_NORMAL, limit);
	else
		read_all_options(context, OT_WHOLE_LINE, limit);
	psql_scan_slash_command_end(context->state);
	return copies;
}

/*
 * Scan one psql document.
 *
 * `names` and `values` are `nvars` parallel arrays of NUL-terminated strings:
 * the psql variables that are set.  The caller owns them.
 */
PgpsoResult *pgpso_scan(const char *source, int source_len,
						const char *const *names, const char *const *values,
						int nvars);

PgpsoResult *
pgpso_scan(const char *source, int source_len,
		   const char *const *names, const char *const *values, int nvars)
{
	PsqlScanCallbacks callbacks;
	PQExpBufferData output;
	PgpsoContext context;
	PgpsoResult *result = calloc(1, sizeof(PgpsoResult));
	PsqlScanState state;
	ConditionalStack lexer_stack;
	int			base = 0;
	int			finished = 0;
	int			i;

	callbacks.get_variable = pgpso_get_variable;
	state = psql_scan_create(&callbacks);

	initPQExpBuffer(&output);
	/* See the file header, adaptation 2. */
	appendPQExpBufferChar(&output, ' ');

	memset(&context, 0, sizeof(context));
	for (i = 0; i < nvars; i++)
		set_variable(&context, names[i], values[i]);
	context.state = state;
	context.output = &output;
	context.result = result;
	context.base = 0;
	context.seed = 1;
	context.cstack = conditional_stack_create();
	context.source = source;
	context.source_len = source_len;
	context.buffer_start = 0;
	context.copy_pos = -1;
	context.copy_line_end = -1;
	current_context = &context;

	/*
	 * psqlscanslash.l evaluates a backquoted argument only when its
	 * passthrough is NULL or an active conditional stack.  This one is never
	 * active, so no shell command runs.
	 */
	lexer_stack = conditional_stack_create();
	conditional_stack_push(lexer_stack, IFSTATE_IGNORED);
	psql_scan_set_passthrough(state, lexer_stack);

	while (!finished)
	{
		/*
		 * The input ends at the end of the line whose sends read COPY data,
		 * and goes on after the data.
		 */
		int			input_end = context.copy_line_end >= 0 ? context.copy_line_end : source_len;

		psql_scan_setup(state, source + base, input_end - base,
						PG_UTF8, true);
		context.base = base;

		for (;;)
		{
			promptStatus_t prompt = PROMPT_READY;
			PsqlScanResult scanned = psql_scan(state, &output, &prompt);
			PgpsoEvent *event;
			int			start;
			int			end;

			if (scanned == PSCAN_SEMICOLON)
			{
				int			copies;
				int			token_end = -1;

				event = push_event(result, PGPSO_SEMICOLON);
				if (current_token(&context, &start, &end))
				{
					event->source_start = start;
					event->source_end = end;
					token_end = end;
				}
				event->output_end = (int) output.len - context.seed;
				event->output_start = event->output_end - 1;
				/* MainLoop sends nothing in an inactive branch. */
				if (!conditional_active(context.cstack))
					continue;
				/* MainLoop: SendQuery, then psql_scan_reset. */
				copies = psql_scan_count_copy_from_stdin(state);
				psql_scan_reset(state);
				context.buffer_start = (int) output.len - context.seed;
				if (copies > 0 && token_end >= 0)
				{
					read_copy_data(&context, token_end, copies);
					if (context.copy_line_end >= 0)
					{
						/* Lex the rest of the line, then the text after the data. */
						psql_scan_finish(state);
						base = token_end;
						break;
					}
				}
				continue;
			}

			if (scanned == PSCAN_BACKSLASH)
			{
				char	   *name;
				int			at = -1;
				int			line_end;
				int			command_end;
				int			name_end;
				int			event_index;
				int			copies;

				if (current_token(&context, &start, &end))
					at = start;
				else
				{
					/*
					 * The backslash came from a substituted value, which has
					 * no place in the document.  Attribute it to the start of
					 * the current input so the scan still advances.
					 */
					at = base;
				}
				line_end = at + 1;
				while (line_end < source_len && source[line_end] != '\n')
					line_end++;

				/*
				 * psql's input buffer ends at the line break (adaptation 1),
				 * so the command and its arguments are read from the rest
				 * of the line only.
				 */
				psql_scan_finish(state);
				psql_scan_setup(state, source + at + 1, line_end - at - 1,
								PG_UTF8, true);
				context.base = at + 1;

				event_index = result->nevents;
				event = push_event(result, PGPSO_BACKSLASH);
				event->source_start = at;
				event->output_start = (int) output.len - context.seed;
				event->output_end = event->output_start;

				name = psql_scan_slash_command(state);
				name_end = at + 1 + (int) strlen(name);
				/* The name stops before the space or the backslash. */
				copies = read_command(&context, name, line_end);
				command_end = slash_position(&context, line_end);
				if (command_end < name_end)
					command_end = name_end;

				/* push_event may have moved the array. */
				event = &result->events[event_index];
				event->source_end = command_end;
				event->detail = 1 + (int) strlen(name);
				event->text = name;

				/*
				 * A command that ends the buffer in an active branch: MainLoop
				 * sends it (and ExecQueryAndProcessResults counts its COPY
				 * commands) and resets the lexer, and `\r` and `\watch` reset
				 * it themselves.
				 */
				if (conditional_active(context.cstack))
				{
					switch (buffer_effect(name))
					{
						case BUFFER_SEND:
							copies = count_copy_of_buffer(&context);
							/* FALLTHROUGH */
						case BUFFER_SEND_NO_COPY:
						case BUFFER_DISCARD:
							psql_scan_reset(state);
							context.buffer_start = (int) output.len - context.seed;
							break;
						case BUFFER_KEEP:
							break;
					}
				}
				read_copy_data(&context, command_end, copies);

				/* Resume SQL lexing where the command ends (adaptation 1). */
				psql_scan_finish(state);
				base = command_end;
				break;
			}

			if (context.copy_line_end >= 0)
			{
				/* The end of the line that read COPY data: go on after it. */
				psql_scan_finish(state);
				base = context.copy_pos;
				context.copy_line_end = -1;
				context.copy_pos = -1;
				break;
			}

			event = push_event(result,
							   scanned == PSCAN_EOL ? PGPSO_EOL : PGPSO_INCOMPLETE);
			event->detail = (int) prompt;
			event->source_start = source_len;
			event->source_end = source_len;
			event->output_start = (int) output.len - context.seed;
			event->output_end = event->output_start;
			psql_scan_finish(state);
			finished = 1;
			break;
		}
	}

	result->sql_len = (int) output.len - context.seed;
	result->sql = malloc((size_t) result->sql_len + 1);
	memcpy(result->sql, output.data + context.seed, (size_t) result->sql_len);
	result->sql[result->sql_len] = '\0';

	termPQExpBuffer(&output);
	psql_scan_destroy(state);
	conditional_stack_destroy(context.cstack);
	conditional_stack_destroy(lexer_stack);
	for (i = 0; i < context.nvars; i++)
	{
		free(context.names[i]);
		free(context.values[i]);
	}
	free(context.names);
	free(context.values);
	current_context = NULL;
	return result;
}

void		pgpso_free(PgpsoResult *result);

void
pgpso_free(PgpsoResult *result)
{
	int			i;

	if (result == NULL)
		return;
	for (i = 0; i < result->nevents; i++)
		free(result->events[i].text);
	free(result->events);
	free(result->sql);
	free(result);
}
