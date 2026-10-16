/*
 * The encoding functions that the static libpq needs.
 *
 * libpq calls pg_char_to_encoding and pg_encoding_to_char. In a libpq shared
 * library they come from libpgcommon_shlib.a. This oracle links the static
 * libpq with libpgcommon.a, as psql does, and from PostgreSQL 17 that library
 * gives the two functions a `_private` spelling (src/include/mb/pg_wchar.h).
 * libpgcommon_shlib.a cannot be linked beside libpgcommon.a, because the two
 * define the same encoding tables. So these definitions give libpq the names
 * it calls. build.rs compiles this file only for a release with the rename.
 */

extern int	pg_char_to_encoding_private(const char *name);
extern const char *pg_encoding_to_char_private(int encoding);

int
pg_char_to_encoding(const char *name)
{
	return pg_char_to_encoding_private(name);
}

const char *
pg_encoding_to_char(int encoding)
{
	return pg_encoding_to_char_private(encoding);
}
