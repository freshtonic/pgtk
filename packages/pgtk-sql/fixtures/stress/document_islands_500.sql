-- batch 0
SELECT id, name FROM accounts WHERE id = 0;
INSERT INTO events (id, kind) VALUES (1, 'k1');
UPDATE accounts SET balance = balance + 2 WHERE id = 2;
DELETE FROM sessions WHERE expires < 3;
CREATE TABLE t4 (id int PRIMARY KEY, v text);
GRANT SELECT ON t5 TO reader;
SET search_path TO s6, public;
COMMIT;
-- batch 1
SELECT id, name FROM accounts WHERE id = 8;
INSERT INTO events (id, kind) VALUES (9, 'k9');
UPDATE accounts SET balance = balance + 10 WHERE id = 10;
DELETE FROM sessions WHERE expires < 11;
CREATE TABLE t12 (id int PRIMARY KEY, v text);
GRANT SELECT ON t13 TO reader;
SET search_path TO s14, public;
COMMIT;
-- batch 2
SELECT id, name FROM accounts WHERE id = 16;
INSERT INTO events (id, kind) VALUES (17, 'k17');
UPDATE accounts SET balance = balance + 18 WHERE id = 18;
DELETE FROM sessions WHERE expires < 19;
CREATE TABLE t20 (id int PRIMARY KEY, v text);
GRANT SELECT ON t21 TO reader;
SET search_path TO s22, public;
COMMIT;
-- batch 3
SELECT id, name FROM accounts WHERE id = 24;
INSERT INTO events (id, kind) VALUES (25, 'k25');
UPDATE accounts SET balance = balance + 26 WHERE id = 26;
DELETE FROM sessions WHERE expires < 27;
CREATE TABLE t28 (id int PRIMARY KEY, v text);
GRANT SELECT ON t29 TO reader;
SET search_path TO s30, public;
COMMIT;
-- batch 4
SELECT id, name FROM accounts WHERE id = 32;
INSERT INTO events (id, kind) VALUES (33, 'k33');
UPDATE accounts SET balance = balance + 34 WHERE id = 34;
DELETE FROM sessions WHERE expires < 35;
CREATE TABLE t36 (id int PRIMARY KEY, v text);
GRANT SELECT ON t37 TO reader;
SET search_path TO s38, public;
COMMIT;
-- batch 5
SELECT id, name FROM accounts WHERE id = 40;
INSERT INTO events (id, kind) VALUES (41, 'k41');
UPDATE accounts SET balance = balance + 42 WHERE id = 42;
DELETE FROM sessions WHERE expires < 43;
CREATE TABLE t44 (id int PRIMARY KEY, v text);
GRANT SELECT ON t45 TO reader;
SET search_path TO s46, public;
COMMIT;
-- batch 6
SELECT id, name FROM accounts WHERE id = 48;
INSERT INTO events (id, kind) VALUES (49, 'k49');
UPDATE accounts SET balance = balance + 50 WHERE id = 50;
DELETE FROM sessions WHERE expires < 51;
CREATE TABLE t52 (id int PRIMARY KEY, v text);
GRANT SELECT ON t53 TO reader;
SET search_path TO s54, public;
COMMIT;
-- batch 7
SELECT id, name FROM accounts WHERE id = 56;
INSERT INTO events (id, kind) VALUES (57, 'k57');
UPDATE accounts SET balance = balance + 58 WHERE id = 58;
DELETE FROM sessions WHERE expires < 59;
CREATE TABLE t60 (id int PRIMARY KEY, v text);
GRANT SELECT ON t61 TO reader;
SET search_path TO s62, public;
COMMIT;
-- batch 8
SELECT id, name FROM accounts WHERE id = 64;
INSERT INTO events (id, kind) VALUES (65, 'k65');
UPDATE accounts SET balance = balance + 66 WHERE id = 66;
DELETE FROM sessions WHERE expires < 67;
CREATE TABLE t68 (id int PRIMARY KEY, v text);
GRANT SELECT ON t69 TO reader;
SET search_path TO s70, public;
COMMIT;
-- batch 9
SELECT id, name FROM accounts WHERE id = 72;
INSERT INTO events (id, kind) VALUES (73, 'k73');
UPDATE accounts SET balance = balance + 74 WHERE id = 74;
DELETE FROM sessions WHERE expires < 75;
CREATE TABLE t76 (id int PRIMARY KEY, v text);
GRANT SELECT ON t77 TO reader;
SET search_path TO s78, public;
COMMIT;
-- batch 10
SELECT id, name FROM accounts WHERE id = 80;
INSERT INTO events (id, kind) VALUES (81, 'k81');
UPDATE accounts SET balance = balance + 82 WHERE id = 82;
DELETE FROM sessions WHERE expires < 83;
CREATE TABLE t84 (id int PRIMARY KEY, v text);
GRANT SELECT ON t85 TO reader;
SET search_path TO s86, public;
COMMIT;
-- batch 11
SELECT id, name FROM accounts WHERE id = 88;
INSERT INTO events (id, kind) VALUES (89, 'k89');
UPDATE accounts SET balance = balance + 90 WHERE id = 90;
DELETE FROM sessions WHERE expires < 91;
CREATE TABLE t92 (id int PRIMARY KEY, v text);
GRANT SELECT ON t93 TO reader;
SET search_path TO s94, public;
COMMIT;
-- batch 12
SELECT id, name FROM accounts WHERE id = 96;
INSERT INTO events (id, kind) VALUES (97, 'k97');
UPDATE accounts SET balance = balance + 98 WHERE id = 98;
DELETE FROM sessions WHERE expires < 99;
CREATE TABLE t100 (id int PRIMARY KEY, v text);
GRANT SELECT ON t101 TO reader;
SET search_path TO s102, public;
COMMIT;
-- batch 13
SELECT id, name FROM accounts WHERE id = 104;
INSERT INTO events (id, kind) VALUES (105, 'k105');
UPDATE accounts SET balance = balance + 106 WHERE id = 106;
DELETE FROM sessions WHERE expires < 107;
CREATE TABLE t108 (id int PRIMARY KEY, v text);
GRANT SELECT ON t109 TO reader;
SET search_path TO s110, public;
COMMIT;
-- batch 14
SELECT id, name FROM accounts WHERE id = 112;
INSERT INTO events (id, kind) VALUES (113, 'k113');
UPDATE accounts SET balance = balance + 114 WHERE id = 114;
DELETE FROM sessions WHERE expires < 115;
CREATE TABLE t116 (id int PRIMARY KEY, v text);
GRANT SELECT ON t117 TO reader;
SET search_path TO s118, public;
COMMIT;
-- batch 15
SELECT id, name FROM accounts WHERE id = 120;
INSERT INTO events (id, kind) VALUES (121, 'k121');
UPDATE accounts SET balance = balance + 122 WHERE id = 122;
DELETE FROM sessions WHERE expires < 123;
CREATE TABLE t124 (id int PRIMARY KEY, v text);
GRANT SELECT ON t125 TO reader;
SET search_path TO s126, public;
COMMIT;
-- batch 16
SELECT id, name FROM accounts WHERE id = 128;
INSERT INTO events (id, kind) VALUES (129, 'k129');
UPDATE accounts SET balance = balance + 130 WHERE id = 130;
DELETE FROM sessions WHERE expires < 131;
CREATE TABLE t132 (id int PRIMARY KEY, v text);
GRANT SELECT ON t133 TO reader;
SET search_path TO s134, public;
COMMIT;
-- batch 17
SELECT id, name FROM accounts WHERE id = 136;
INSERT INTO events (id, kind) VALUES (137, 'k137');
UPDATE accounts SET balance = balance + 138 WHERE id = 138;
DELETE FROM sessions WHERE expires < 139;
CREATE TABLE t140 (id int PRIMARY KEY, v text);
GRANT SELECT ON t141 TO reader;
SET search_path TO s142, public;
COMMIT;
-- batch 18
SELECT id, name FROM accounts WHERE id = 144;
INSERT INTO events (id, kind) VALUES (145, 'k145');
UPDATE accounts SET balance = balance + 146 WHERE id = 146;
DELETE FROM sessions WHERE expires < 147;
CREATE TABLE t148 (id int PRIMARY KEY, v text);
GRANT SELECT ON t149 TO reader;
SET search_path TO s150, public;
COMMIT;
-- batch 19
SELECT id, name FROM accounts WHERE id = 152;
INSERT INTO events (id, kind) VALUES (153, 'k153');
UPDATE accounts SET balance = balance + 154 WHERE id = 154;
DELETE FROM sessions WHERE expires < 155;
CREATE TABLE t156 (id int PRIMARY KEY, v text);
GRANT SELECT ON t157 TO reader;
SET search_path TO s158, public;
COMMIT;
-- batch 20
SELECT id, name FROM accounts WHERE id = 160;
INSERT INTO events (id, kind) VALUES (161, 'k161');
UPDATE accounts SET balance = balance + 162 WHERE id = 162;
DELETE FROM sessions WHERE expires < 163;
CREATE TABLE t164 (id int PRIMARY KEY, v text);
GRANT SELECT ON t165 TO reader;
SET search_path TO s166, public;
COMMIT;
-- batch 21
SELECT id, name FROM accounts WHERE id = 168;
INSERT INTO events (id, kind) VALUES (169, 'k169');
UPDATE accounts SET balance = balance + 170 WHERE id = 170;
DELETE FROM sessions WHERE expires < 171;
CREATE TABLE t172 (id int PRIMARY KEY, v text);
GRANT SELECT ON t173 TO reader;
SET search_path TO s174, public;
COMMIT;
-- batch 22
SELECT id, name FROM accounts WHERE id = 176;
INSERT INTO events (id, kind) VALUES (177, 'k177');
UPDATE accounts SET balance = balance + 178 WHERE id = 178;
DELETE FROM sessions WHERE expires < 179;
CREATE TABLE t180 (id int PRIMARY KEY, v text);
GRANT SELECT ON t181 TO reader;
SET search_path TO s182, public;
COMMIT;
-- batch 23
SELECT id, name FROM accounts WHERE id = 184;
INSERT INTO events (id, kind) VALUES (185, 'k185');
UPDATE accounts SET balance = balance + 186 WHERE id = 186;
DELETE FROM sessions WHERE expires < 187;
CREATE TABLE t188 (id int PRIMARY KEY, v text);
GRANT SELECT ON t189 TO reader;
SET search_path TO s190, public;
COMMIT;
-- batch 24
SELECT id, name FROM accounts WHERE id = 192;
INSERT INTO events (id, kind) VALUES (193, 'k193');
UPDATE accounts SET balance = balance + 194 WHERE id = 194;
DELETE FROM sessions WHERE expires < 195;
CREATE TABLE t196 (id int PRIMARY KEY, v text);
GRANT SELECT ON t197 TO reader;
SET search_path TO s198, public;
COMMIT;
-- batch 25
SELECT id, name FROM accounts WHERE id = 200;
INSERT INTO events (id, kind) VALUES (201, 'k201');
UPDATE accounts SET balance = balance + 202 WHERE id = 202;
DELETE FROM sessions WHERE expires < 203;
CREATE TABLE t204 (id int PRIMARY KEY, v text);
GRANT SELECT ON t205 TO reader;
SET search_path TO s206, public;
COMMIT;
-- batch 26
SELECT id, name FROM accounts WHERE id = 208;
INSERT INTO events (id, kind) VALUES (209, 'k209');
UPDATE accounts SET balance = balance + 210 WHERE id = 210;
DELETE FROM sessions WHERE expires < 211;
CREATE TABLE t212 (id int PRIMARY KEY, v text);
GRANT SELECT ON t213 TO reader;
SET search_path TO s214, public;
COMMIT;
-- batch 27
SELECT id, name FROM accounts WHERE id = 216;
INSERT INTO events (id, kind) VALUES (217, 'k217');
UPDATE accounts SET balance = balance + 218 WHERE id = 218;
DELETE FROM sessions WHERE expires < 219;
CREATE TABLE t220 (id int PRIMARY KEY, v text);
GRANT SELECT ON t221 TO reader;
SET search_path TO s222, public;
COMMIT;
-- batch 28
SELECT id, name FROM accounts WHERE id = 224;
INSERT INTO events (id, kind) VALUES (225, 'k225');
UPDATE accounts SET balance = balance + 226 WHERE id = 226;
DELETE FROM sessions WHERE expires < 227;
CREATE TABLE t228 (id int PRIMARY KEY, v text);
GRANT SELECT ON t229 TO reader;
SET search_path TO s230, public;
COMMIT;
-- batch 29
SELECT id, name FROM accounts WHERE id = 232;
INSERT INTO events (id, kind) VALUES (233, 'k233');
UPDATE accounts SET balance = balance + 234 WHERE id = 234;
DELETE FROM sessions WHERE expires < 235;
CREATE TABLE t236 (id int PRIMARY KEY, v text);
GRANT SELECT ON t237 TO reader;
SET search_path TO s238, public;
COMMIT;
-- batch 30
SELECT id, name FROM accounts WHERE id = 240;
INSERT INTO events (id, kind) VALUES (241, 'k241');
UPDATE accounts SET balance = balance + 242 WHERE id = 242;
DELETE FROM sessions WHERE expires < 243;
CREATE TABLE t244 (id int PRIMARY KEY, v text);
GRANT SELECT ON t245 TO reader;
SET search_path TO s246, public;
COMMIT;
-- batch 31
SELECT id, name FROM accounts WHERE id = 248;
INSERT INTO events (id, kind) VALUES (249, 'k249');
UPDATE accounts SET balance = balance + 250 WHERE id = 250;
DELETE FROM sessions WHERE expires < 251;
CREATE TABLE t252 (id int PRIMARY KEY, v text);
GRANT SELECT ON t253 TO reader;
SET search_path TO s254, public;
COMMIT;
-- batch 32
SELECT id, name FROM accounts WHERE id = 256;
INSERT INTO events (id, kind) VALUES (257, 'k257');
UPDATE accounts SET balance = balance + 258 WHERE id = 258;
DELETE FROM sessions WHERE expires < 259;
CREATE TABLE t260 (id int PRIMARY KEY, v text);
GRANT SELECT ON t261 TO reader;
SET search_path TO s262, public;
COMMIT;
-- batch 33
SELECT id, name FROM accounts WHERE id = 264;
INSERT INTO events (id, kind) VALUES (265, 'k265');
UPDATE accounts SET balance = balance + 266 WHERE id = 266;
DELETE FROM sessions WHERE expires < 267;
CREATE TABLE t268 (id int PRIMARY KEY, v text);
GRANT SELECT ON t269 TO reader;
SET search_path TO s270, public;
COMMIT;
-- batch 34
SELECT id, name FROM accounts WHERE id = 272;
INSERT INTO events (id, kind) VALUES (273, 'k273');
UPDATE accounts SET balance = balance + 274 WHERE id = 274;
DELETE FROM sessions WHERE expires < 275;
CREATE TABLE t276 (id int PRIMARY KEY, v text);
GRANT SELECT ON t277 TO reader;
SET search_path TO s278, public;
COMMIT;
-- batch 35
SELECT id, name FROM accounts WHERE id = 280;
INSERT INTO events (id, kind) VALUES (281, 'k281');
UPDATE accounts SET balance = balance + 282 WHERE id = 282;
DELETE FROM sessions WHERE expires < 283;
CREATE TABLE t284 (id int PRIMARY KEY, v text);
GRANT SELECT ON t285 TO reader;
SET search_path TO s286, public;
COMMIT;
-- batch 36
SELECT id, name FROM accounts WHERE id = 288;
INSERT INTO events (id, kind) VALUES (289, 'k289');
UPDATE accounts SET balance = balance + 290 WHERE id = 290;
DELETE FROM sessions WHERE expires < 291;
CREATE TABLE t292 (id int PRIMARY KEY, v text);
GRANT SELECT ON t293 TO reader;
SET search_path TO s294, public;
COMMIT;
-- batch 37
SELECT id, name FROM accounts WHERE id = 296;
INSERT INTO events (id, kind) VALUES (297, 'k297');
UPDATE accounts SET balance = balance + 298 WHERE id = 298;
DELETE FROM sessions WHERE expires < 299;
CREATE TABLE t300 (id int PRIMARY KEY, v text);
GRANT SELECT ON t301 TO reader;
SET search_path TO s302, public;
COMMIT;
-- batch 38
SELECT id, name FROM accounts WHERE id = 304;
INSERT INTO events (id, kind) VALUES (305, 'k305');
UPDATE accounts SET balance = balance + 306 WHERE id = 306;
DELETE FROM sessions WHERE expires < 307;
CREATE TABLE t308 (id int PRIMARY KEY, v text);
GRANT SELECT ON t309 TO reader;
SET search_path TO s310, public;
COMMIT;
-- batch 39
SELECT id, name FROM accounts WHERE id = 312;
INSERT INTO events (id, kind) VALUES (313, 'k313');
UPDATE accounts SET balance = balance + 314 WHERE id = 314;
DELETE FROM sessions WHERE expires < 315;
CREATE TABLE t316 (id int PRIMARY KEY, v text);
GRANT SELECT ON t317 TO reader;
SET search_path TO s318, public;
COMMIT;
-- batch 40
SELECT id, name FROM accounts WHERE id = 320;
INSERT INTO events (id, kind) VALUES (321, 'k321');
UPDATE accounts SET balance = balance + 322 WHERE id = 322;
DELETE FROM sessions WHERE expires < 323;
CREATE TABLE t324 (id int PRIMARY KEY, v text);
GRANT SELECT ON t325 TO reader;
SET search_path TO s326, public;
COMMIT;
-- batch 41
SELECT id, name FROM accounts WHERE id = 328;
INSERT INTO events (id, kind) VALUES (329, 'k329');
UPDATE accounts SET balance = balance + 330 WHERE id = 330;
DELETE FROM sessions WHERE expires < 331;
CREATE TABLE t332 (id int PRIMARY KEY, v text);
GRANT SELECT ON t333 TO reader;
SET search_path TO s334, public;
COMMIT;
-- batch 42
SELECT id, name FROM accounts WHERE id = 336;
INSERT INTO events (id, kind) VALUES (337, 'k337');
UPDATE accounts SET balance = balance + 338 WHERE id = 338;
DELETE FROM sessions WHERE expires < 339;
CREATE TABLE t340 (id int PRIMARY KEY, v text);
GRANT SELECT ON t341 TO reader;
SET search_path TO s342, public;
COMMIT;
-- batch 43
SELECT id, name FROM accounts WHERE id = 344;
INSERT INTO events (id, kind) VALUES (345, 'k345');
UPDATE accounts SET balance = balance + 346 WHERE id = 346;
DELETE FROM sessions WHERE expires < 347;
CREATE TABLE t348 (id int PRIMARY KEY, v text);
GRANT SELECT ON t349 TO reader;
SET search_path TO s350, public;
COMMIT;
-- batch 44
SELECT id, name FROM accounts WHERE id = 352;
INSERT INTO events (id, kind) VALUES (353, 'k353');
UPDATE accounts SET balance = balance + 354 WHERE id = 354;
DELETE FROM sessions WHERE expires < 355;
CREATE TABLE t356 (id int PRIMARY KEY, v text);
GRANT SELECT ON t357 TO reader;
SET search_path TO s358, public;
COMMIT;
-- batch 45
SELECT id, name FROM accounts WHERE id = 360;
INSERT INTO events (id, kind) VALUES (361, 'k361');
UPDATE accounts SET balance = balance + 362 WHERE id = 362;
DELETE FROM sessions WHERE expires < 363;
CREATE TABLE t364 (id int PRIMARY KEY, v text);
GRANT SELECT ON t365 TO reader;
SET search_path TO s366, public;
COMMIT;
-- batch 46
SELECT id, name FROM accounts WHERE id = 368;
INSERT INTO events (id, kind) VALUES (369, 'k369');
UPDATE accounts SET balance = balance + 370 WHERE id = 370;
DELETE FROM sessions WHERE expires < 371;
CREATE TABLE t372 (id int PRIMARY KEY, v text);
GRANT SELECT ON t373 TO reader;
SET search_path TO s374, public;
COMMIT;
-- batch 47
SELECT id, name FROM accounts WHERE id = 376;
INSERT INTO events (id, kind) VALUES (377, 'k377');
UPDATE accounts SET balance = balance + 378 WHERE id = 378;
DELETE FROM sessions WHERE expires < 379;
CREATE TABLE t380 (id int PRIMARY KEY, v text);
GRANT SELECT ON t381 TO reader;
SET search_path TO s382, public;
COMMIT;
-- batch 48
SELECT id, name FROM accounts WHERE id = 384;
INSERT INTO events (id, kind) VALUES (385, 'k385');
UPDATE accounts SET balance = balance + 386 WHERE id = 386;
DELETE FROM sessions WHERE expires < 387;
CREATE TABLE t388 (id int PRIMARY KEY, v text);
GRANT SELECT ON t389 TO reader;
SET search_path TO s390, public;
COMMIT;
-- batch 49
SELECT id, name FROM accounts WHERE id = 392;
INSERT INTO events (id, kind) VALUES (393, 'k393');
UPDATE accounts SET balance = balance + 394 WHERE id = 394;
DELETE FROM sessions WHERE expires < 395;
CREATE TABLE t396 (id int PRIMARY KEY, v text);
GRANT SELECT ON t397 TO reader;
SET search_path TO s398, public;
COMMIT;
-- batch 50
SELECT id, name FROM accounts WHERE id = 400;
INSERT INTO events (id, kind) VALUES (401, 'k401');
UPDATE accounts SET balance = balance + 402 WHERE id = 402;
DELETE FROM sessions WHERE expires < 403;
CREATE TABLE t404 (id int PRIMARY KEY, v text);
GRANT SELECT ON t405 TO reader;
SET search_path TO s406, public;
COMMIT;
-- batch 51
SELECT id, name FROM accounts WHERE id = 408;
INSERT INTO events (id, kind) VALUES (409, 'k409');
UPDATE accounts SET balance = balance + 410 WHERE id = 410;
DELETE FROM sessions WHERE expires < 411;
CREATE TABLE t412 (id int PRIMARY KEY, v text);
GRANT SELECT ON t413 TO reader;
SET search_path TO s414, public;
COMMIT;
-- batch 52
SELECT id, name FROM accounts WHERE id = 416;
INSERT INTO events (id, kind) VALUES (417, 'k417');
UPDATE accounts SET balance = balance + 418 WHERE id = 418;
DELETE FROM sessions WHERE expires < 419;
CREATE TABLE t420 (id int PRIMARY KEY, v text);
GRANT SELECT ON t421 TO reader;
SET search_path TO s422, public;
COMMIT;
-- batch 53
SELECT id, name FROM accounts WHERE id = 424;
INSERT INTO events (id, kind) VALUES (425, 'k425');
UPDATE accounts SET balance = balance + 426 WHERE id = 426;
DELETE FROM sessions WHERE expires < 427;
CREATE TABLE t428 (id int PRIMARY KEY, v text);
GRANT SELECT ON t429 TO reader;
SET search_path TO s430, public;
COMMIT;
-- batch 54
SELECT id, name FROM accounts WHERE id = 432;
INSERT INTO events (id, kind) VALUES (433, 'k433');
UPDATE accounts SET balance = balance + 434 WHERE id = 434;
DELETE FROM sessions WHERE expires < 435;
CREATE TABLE t436 (id int PRIMARY KEY, v text);
GRANT SELECT ON t437 TO reader;
SET search_path TO s438, public;
COMMIT;
-- batch 55
SELECT id, name FROM accounts WHERE id = 440;
INSERT INTO events (id, kind) VALUES (441, 'k441');
UPDATE accounts SET balance = balance + 442 WHERE id = 442;
DELETE FROM sessions WHERE expires < 443;
CREATE TABLE t444 (id int PRIMARY KEY, v text);
GRANT SELECT ON t445 TO reader;
SET search_path TO s446, public;
COMMIT;
-- batch 56
SELECT id, name FROM accounts WHERE id = 448;
INSERT INTO events (id, kind) VALUES (449, 'k449');
UPDATE accounts SET balance = balance + 450 WHERE id = 450;
DELETE FROM sessions WHERE expires < 451;
CREATE TABLE t452 (id int PRIMARY KEY, v text);
GRANT SELECT ON t453 TO reader;
SET search_path TO s454, public;
COMMIT;
-- batch 57
SELECT id, name FROM accounts WHERE id = 456;
INSERT INTO events (id, kind) VALUES (457, 'k457');
UPDATE accounts SET balance = balance + 458 WHERE id = 458;
DELETE FROM sessions WHERE expires < 459;
CREATE TABLE t460 (id int PRIMARY KEY, v text);
GRANT SELECT ON t461 TO reader;
SET search_path TO s462, public;
COMMIT;
-- batch 58
SELECT id, name FROM accounts WHERE id = 464;
INSERT INTO events (id, kind) VALUES (465, 'k465');
UPDATE accounts SET balance = balance + 466 WHERE id = 466;
DELETE FROM sessions WHERE expires < 467;
CREATE TABLE t468 (id int PRIMARY KEY, v text);
GRANT SELECT ON t469 TO reader;
SET search_path TO s470, public;
COMMIT;
-- batch 59
SELECT id, name FROM accounts WHERE id = 472;
INSERT INTO events (id, kind) VALUES (473, 'k473');
UPDATE accounts SET balance = balance + 474 WHERE id = 474;
DELETE FROM sessions WHERE expires < 475;
CREATE TABLE t476 (id int PRIMARY KEY, v text);
GRANT SELECT ON t477 TO reader;
SET search_path TO s478, public;
COMMIT;
-- batch 60
SELECT id, name FROM accounts WHERE id = 480;
INSERT INTO events (id, kind) VALUES (481, 'k481');
UPDATE accounts SET balance = balance + 482 WHERE id = 482;
DELETE FROM sessions WHERE expires < 483;
CREATE TABLE t484 (id int PRIMARY KEY, v text);
GRANT SELECT ON t485 TO reader;
SET search_path TO s486, public;
COMMIT;
-- batch 61
SELECT id, name FROM accounts WHERE id = 488;
INSERT INTO events (id, kind) VALUES (489, 'k489');
UPDATE accounts SET balance = balance + 490 WHERE id = 490;
DELETE FROM sessions WHERE expires < 491;
CREATE TABLE t492 (id int PRIMARY KEY, v text);
GRANT SELECT ON t493 TO reader;
SET search_path TO s494, public;
COMMIT;
-- batch 62
SELECT id, name FROM accounts WHERE id = 496;
INSERT INTO events (id, kind) VALUES (497, 'k497');
UPDATE accounts SET balance = balance + 498 WHERE id = 498;
DELETE FROM sessions WHERE expires < 499;
