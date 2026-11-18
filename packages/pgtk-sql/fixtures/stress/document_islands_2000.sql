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
CREATE TABLE t500 (id int PRIMARY KEY, v text);
GRANT SELECT ON t501 TO reader;
SET search_path TO s502, public;
COMMIT;
-- batch 63
SELECT id, name FROM accounts WHERE id = 504;
INSERT INTO events (id, kind) VALUES (505, 'k505');
UPDATE accounts SET balance = balance + 506 WHERE id = 506;
DELETE FROM sessions WHERE expires < 507;
CREATE TABLE t508 (id int PRIMARY KEY, v text);
GRANT SELECT ON t509 TO reader;
SET search_path TO s510, public;
COMMIT;
-- batch 64
SELECT id, name FROM accounts WHERE id = 512;
INSERT INTO events (id, kind) VALUES (513, 'k513');
UPDATE accounts SET balance = balance + 514 WHERE id = 514;
DELETE FROM sessions WHERE expires < 515;
CREATE TABLE t516 (id int PRIMARY KEY, v text);
GRANT SELECT ON t517 TO reader;
SET search_path TO s518, public;
COMMIT;
-- batch 65
SELECT id, name FROM accounts WHERE id = 520;
INSERT INTO events (id, kind) VALUES (521, 'k521');
UPDATE accounts SET balance = balance + 522 WHERE id = 522;
DELETE FROM sessions WHERE expires < 523;
CREATE TABLE t524 (id int PRIMARY KEY, v text);
GRANT SELECT ON t525 TO reader;
SET search_path TO s526, public;
COMMIT;
-- batch 66
SELECT id, name FROM accounts WHERE id = 528;
INSERT INTO events (id, kind) VALUES (529, 'k529');
UPDATE accounts SET balance = balance + 530 WHERE id = 530;
DELETE FROM sessions WHERE expires < 531;
CREATE TABLE t532 (id int PRIMARY KEY, v text);
GRANT SELECT ON t533 TO reader;
SET search_path TO s534, public;
COMMIT;
-- batch 67
SELECT id, name FROM accounts WHERE id = 536;
INSERT INTO events (id, kind) VALUES (537, 'k537');
UPDATE accounts SET balance = balance + 538 WHERE id = 538;
DELETE FROM sessions WHERE expires < 539;
CREATE TABLE t540 (id int PRIMARY KEY, v text);
GRANT SELECT ON t541 TO reader;
SET search_path TO s542, public;
COMMIT;
-- batch 68
SELECT id, name FROM accounts WHERE id = 544;
INSERT INTO events (id, kind) VALUES (545, 'k545');
UPDATE accounts SET balance = balance + 546 WHERE id = 546;
DELETE FROM sessions WHERE expires < 547;
CREATE TABLE t548 (id int PRIMARY KEY, v text);
GRANT SELECT ON t549 TO reader;
SET search_path TO s550, public;
COMMIT;
-- batch 69
SELECT id, name FROM accounts WHERE id = 552;
INSERT INTO events (id, kind) VALUES (553, 'k553');
UPDATE accounts SET balance = balance + 554 WHERE id = 554;
DELETE FROM sessions WHERE expires < 555;
CREATE TABLE t556 (id int PRIMARY KEY, v text);
GRANT SELECT ON t557 TO reader;
SET search_path TO s558, public;
COMMIT;
-- batch 70
SELECT id, name FROM accounts WHERE id = 560;
INSERT INTO events (id, kind) VALUES (561, 'k561');
UPDATE accounts SET balance = balance + 562 WHERE id = 562;
DELETE FROM sessions WHERE expires < 563;
CREATE TABLE t564 (id int PRIMARY KEY, v text);
GRANT SELECT ON t565 TO reader;
SET search_path TO s566, public;
COMMIT;
-- batch 71
SELECT id, name FROM accounts WHERE id = 568;
INSERT INTO events (id, kind) VALUES (569, 'k569');
UPDATE accounts SET balance = balance + 570 WHERE id = 570;
DELETE FROM sessions WHERE expires < 571;
CREATE TABLE t572 (id int PRIMARY KEY, v text);
GRANT SELECT ON t573 TO reader;
SET search_path TO s574, public;
COMMIT;
-- batch 72
SELECT id, name FROM accounts WHERE id = 576;
INSERT INTO events (id, kind) VALUES (577, 'k577');
UPDATE accounts SET balance = balance + 578 WHERE id = 578;
DELETE FROM sessions WHERE expires < 579;
CREATE TABLE t580 (id int PRIMARY KEY, v text);
GRANT SELECT ON t581 TO reader;
SET search_path TO s582, public;
COMMIT;
-- batch 73
SELECT id, name FROM accounts WHERE id = 584;
INSERT INTO events (id, kind) VALUES (585, 'k585');
UPDATE accounts SET balance = balance + 586 WHERE id = 586;
DELETE FROM sessions WHERE expires < 587;
CREATE TABLE t588 (id int PRIMARY KEY, v text);
GRANT SELECT ON t589 TO reader;
SET search_path TO s590, public;
COMMIT;
-- batch 74
SELECT id, name FROM accounts WHERE id = 592;
INSERT INTO events (id, kind) VALUES (593, 'k593');
UPDATE accounts SET balance = balance + 594 WHERE id = 594;
DELETE FROM sessions WHERE expires < 595;
CREATE TABLE t596 (id int PRIMARY KEY, v text);
GRANT SELECT ON t597 TO reader;
SET search_path TO s598, public;
COMMIT;
-- batch 75
SELECT id, name FROM accounts WHERE id = 600;
INSERT INTO events (id, kind) VALUES (601, 'k601');
UPDATE accounts SET balance = balance + 602 WHERE id = 602;
DELETE FROM sessions WHERE expires < 603;
CREATE TABLE t604 (id int PRIMARY KEY, v text);
GRANT SELECT ON t605 TO reader;
SET search_path TO s606, public;
COMMIT;
-- batch 76
SELECT id, name FROM accounts WHERE id = 608;
INSERT INTO events (id, kind) VALUES (609, 'k609');
UPDATE accounts SET balance = balance + 610 WHERE id = 610;
DELETE FROM sessions WHERE expires < 611;
CREATE TABLE t612 (id int PRIMARY KEY, v text);
GRANT SELECT ON t613 TO reader;
SET search_path TO s614, public;
COMMIT;
-- batch 77
SELECT id, name FROM accounts WHERE id = 616;
INSERT INTO events (id, kind) VALUES (617, 'k617');
UPDATE accounts SET balance = balance + 618 WHERE id = 618;
DELETE FROM sessions WHERE expires < 619;
CREATE TABLE t620 (id int PRIMARY KEY, v text);
GRANT SELECT ON t621 TO reader;
SET search_path TO s622, public;
COMMIT;
-- batch 78
SELECT id, name FROM accounts WHERE id = 624;
INSERT INTO events (id, kind) VALUES (625, 'k625');
UPDATE accounts SET balance = balance + 626 WHERE id = 626;
DELETE FROM sessions WHERE expires < 627;
CREATE TABLE t628 (id int PRIMARY KEY, v text);
GRANT SELECT ON t629 TO reader;
SET search_path TO s630, public;
COMMIT;
-- batch 79
SELECT id, name FROM accounts WHERE id = 632;
INSERT INTO events (id, kind) VALUES (633, 'k633');
UPDATE accounts SET balance = balance + 634 WHERE id = 634;
DELETE FROM sessions WHERE expires < 635;
CREATE TABLE t636 (id int PRIMARY KEY, v text);
GRANT SELECT ON t637 TO reader;
SET search_path TO s638, public;
COMMIT;
-- batch 80
SELECT id, name FROM accounts WHERE id = 640;
INSERT INTO events (id, kind) VALUES (641, 'k641');
UPDATE accounts SET balance = balance + 642 WHERE id = 642;
DELETE FROM sessions WHERE expires < 643;
CREATE TABLE t644 (id int PRIMARY KEY, v text);
GRANT SELECT ON t645 TO reader;
SET search_path TO s646, public;
COMMIT;
-- batch 81
SELECT id, name FROM accounts WHERE id = 648;
INSERT INTO events (id, kind) VALUES (649, 'k649');
UPDATE accounts SET balance = balance + 650 WHERE id = 650;
DELETE FROM sessions WHERE expires < 651;
CREATE TABLE t652 (id int PRIMARY KEY, v text);
GRANT SELECT ON t653 TO reader;
SET search_path TO s654, public;
COMMIT;
-- batch 82
SELECT id, name FROM accounts WHERE id = 656;
INSERT INTO events (id, kind) VALUES (657, 'k657');
UPDATE accounts SET balance = balance + 658 WHERE id = 658;
DELETE FROM sessions WHERE expires < 659;
CREATE TABLE t660 (id int PRIMARY KEY, v text);
GRANT SELECT ON t661 TO reader;
SET search_path TO s662, public;
COMMIT;
-- batch 83
SELECT id, name FROM accounts WHERE id = 664;
INSERT INTO events (id, kind) VALUES (665, 'k665');
UPDATE accounts SET balance = balance + 666 WHERE id = 666;
DELETE FROM sessions WHERE expires < 667;
CREATE TABLE t668 (id int PRIMARY KEY, v text);
GRANT SELECT ON t669 TO reader;
SET search_path TO s670, public;
COMMIT;
-- batch 84
SELECT id, name FROM accounts WHERE id = 672;
INSERT INTO events (id, kind) VALUES (673, 'k673');
UPDATE accounts SET balance = balance + 674 WHERE id = 674;
DELETE FROM sessions WHERE expires < 675;
CREATE TABLE t676 (id int PRIMARY KEY, v text);
GRANT SELECT ON t677 TO reader;
SET search_path TO s678, public;
COMMIT;
-- batch 85
SELECT id, name FROM accounts WHERE id = 680;
INSERT INTO events (id, kind) VALUES (681, 'k681');
UPDATE accounts SET balance = balance + 682 WHERE id = 682;
DELETE FROM sessions WHERE expires < 683;
CREATE TABLE t684 (id int PRIMARY KEY, v text);
GRANT SELECT ON t685 TO reader;
SET search_path TO s686, public;
COMMIT;
-- batch 86
SELECT id, name FROM accounts WHERE id = 688;
INSERT INTO events (id, kind) VALUES (689, 'k689');
UPDATE accounts SET balance = balance + 690 WHERE id = 690;
DELETE FROM sessions WHERE expires < 691;
CREATE TABLE t692 (id int PRIMARY KEY, v text);
GRANT SELECT ON t693 TO reader;
SET search_path TO s694, public;
COMMIT;
-- batch 87
SELECT id, name FROM accounts WHERE id = 696;
INSERT INTO events (id, kind) VALUES (697, 'k697');
UPDATE accounts SET balance = balance + 698 WHERE id = 698;
DELETE FROM sessions WHERE expires < 699;
CREATE TABLE t700 (id int PRIMARY KEY, v text);
GRANT SELECT ON t701 TO reader;
SET search_path TO s702, public;
COMMIT;
-- batch 88
SELECT id, name FROM accounts WHERE id = 704;
INSERT INTO events (id, kind) VALUES (705, 'k705');
UPDATE accounts SET balance = balance + 706 WHERE id = 706;
DELETE FROM sessions WHERE expires < 707;
CREATE TABLE t708 (id int PRIMARY KEY, v text);
GRANT SELECT ON t709 TO reader;
SET search_path TO s710, public;
COMMIT;
-- batch 89
SELECT id, name FROM accounts WHERE id = 712;
INSERT INTO events (id, kind) VALUES (713, 'k713');
UPDATE accounts SET balance = balance + 714 WHERE id = 714;
DELETE FROM sessions WHERE expires < 715;
CREATE TABLE t716 (id int PRIMARY KEY, v text);
GRANT SELECT ON t717 TO reader;
SET search_path TO s718, public;
COMMIT;
-- batch 90
SELECT id, name FROM accounts WHERE id = 720;
INSERT INTO events (id, kind) VALUES (721, 'k721');
UPDATE accounts SET balance = balance + 722 WHERE id = 722;
DELETE FROM sessions WHERE expires < 723;
CREATE TABLE t724 (id int PRIMARY KEY, v text);
GRANT SELECT ON t725 TO reader;
SET search_path TO s726, public;
COMMIT;
-- batch 91
SELECT id, name FROM accounts WHERE id = 728;
INSERT INTO events (id, kind) VALUES (729, 'k729');
UPDATE accounts SET balance = balance + 730 WHERE id = 730;
DELETE FROM sessions WHERE expires < 731;
CREATE TABLE t732 (id int PRIMARY KEY, v text);
GRANT SELECT ON t733 TO reader;
SET search_path TO s734, public;
COMMIT;
-- batch 92
SELECT id, name FROM accounts WHERE id = 736;
INSERT INTO events (id, kind) VALUES (737, 'k737');
UPDATE accounts SET balance = balance + 738 WHERE id = 738;
DELETE FROM sessions WHERE expires < 739;
CREATE TABLE t740 (id int PRIMARY KEY, v text);
GRANT SELECT ON t741 TO reader;
SET search_path TO s742, public;
COMMIT;
-- batch 93
SELECT id, name FROM accounts WHERE id = 744;
INSERT INTO events (id, kind) VALUES (745, 'k745');
UPDATE accounts SET balance = balance + 746 WHERE id = 746;
DELETE FROM sessions WHERE expires < 747;
CREATE TABLE t748 (id int PRIMARY KEY, v text);
GRANT SELECT ON t749 TO reader;
SET search_path TO s750, public;
COMMIT;
-- batch 94
SELECT id, name FROM accounts WHERE id = 752;
INSERT INTO events (id, kind) VALUES (753, 'k753');
UPDATE accounts SET balance = balance + 754 WHERE id = 754;
DELETE FROM sessions WHERE expires < 755;
CREATE TABLE t756 (id int PRIMARY KEY, v text);
GRANT SELECT ON t757 TO reader;
SET search_path TO s758, public;
COMMIT;
-- batch 95
SELECT id, name FROM accounts WHERE id = 760;
INSERT INTO events (id, kind) VALUES (761, 'k761');
UPDATE accounts SET balance = balance + 762 WHERE id = 762;
DELETE FROM sessions WHERE expires < 763;
CREATE TABLE t764 (id int PRIMARY KEY, v text);
GRANT SELECT ON t765 TO reader;
SET search_path TO s766, public;
COMMIT;
-- batch 96
SELECT id, name FROM accounts WHERE id = 768;
INSERT INTO events (id, kind) VALUES (769, 'k769');
UPDATE accounts SET balance = balance + 770 WHERE id = 770;
DELETE FROM sessions WHERE expires < 771;
CREATE TABLE t772 (id int PRIMARY KEY, v text);
GRANT SELECT ON t773 TO reader;
SET search_path TO s774, public;
COMMIT;
-- batch 97
SELECT id, name FROM accounts WHERE id = 776;
INSERT INTO events (id, kind) VALUES (777, 'k777');
UPDATE accounts SET balance = balance + 778 WHERE id = 778;
DELETE FROM sessions WHERE expires < 779;
CREATE TABLE t780 (id int PRIMARY KEY, v text);
GRANT SELECT ON t781 TO reader;
SET search_path TO s782, public;
COMMIT;
-- batch 98
SELECT id, name FROM accounts WHERE id = 784;
INSERT INTO events (id, kind) VALUES (785, 'k785');
UPDATE accounts SET balance = balance + 786 WHERE id = 786;
DELETE FROM sessions WHERE expires < 787;
CREATE TABLE t788 (id int PRIMARY KEY, v text);
GRANT SELECT ON t789 TO reader;
SET search_path TO s790, public;
COMMIT;
-- batch 99
SELECT id, name FROM accounts WHERE id = 792;
INSERT INTO events (id, kind) VALUES (793, 'k793');
UPDATE accounts SET balance = balance + 794 WHERE id = 794;
DELETE FROM sessions WHERE expires < 795;
CREATE TABLE t796 (id int PRIMARY KEY, v text);
GRANT SELECT ON t797 TO reader;
SET search_path TO s798, public;
COMMIT;
-- batch 100
SELECT id, name FROM accounts WHERE id = 800;
INSERT INTO events (id, kind) VALUES (801, 'k801');
UPDATE accounts SET balance = balance + 802 WHERE id = 802;
DELETE FROM sessions WHERE expires < 803;
CREATE TABLE t804 (id int PRIMARY KEY, v text);
GRANT SELECT ON t805 TO reader;
SET search_path TO s806, public;
COMMIT;
-- batch 101
SELECT id, name FROM accounts WHERE id = 808;
INSERT INTO events (id, kind) VALUES (809, 'k809');
UPDATE accounts SET balance = balance + 810 WHERE id = 810;
DELETE FROM sessions WHERE expires < 811;
CREATE TABLE t812 (id int PRIMARY KEY, v text);
GRANT SELECT ON t813 TO reader;
SET search_path TO s814, public;
COMMIT;
-- batch 102
SELECT id, name FROM accounts WHERE id = 816;
INSERT INTO events (id, kind) VALUES (817, 'k817');
UPDATE accounts SET balance = balance + 818 WHERE id = 818;
DELETE FROM sessions WHERE expires < 819;
CREATE TABLE t820 (id int PRIMARY KEY, v text);
GRANT SELECT ON t821 TO reader;
SET search_path TO s822, public;
COMMIT;
-- batch 103
SELECT id, name FROM accounts WHERE id = 824;
INSERT INTO events (id, kind) VALUES (825, 'k825');
UPDATE accounts SET balance = balance + 826 WHERE id = 826;
DELETE FROM sessions WHERE expires < 827;
CREATE TABLE t828 (id int PRIMARY KEY, v text);
GRANT SELECT ON t829 TO reader;
SET search_path TO s830, public;
COMMIT;
-- batch 104
SELECT id, name FROM accounts WHERE id = 832;
INSERT INTO events (id, kind) VALUES (833, 'k833');
UPDATE accounts SET balance = balance + 834 WHERE id = 834;
DELETE FROM sessions WHERE expires < 835;
CREATE TABLE t836 (id int PRIMARY KEY, v text);
GRANT SELECT ON t837 TO reader;
SET search_path TO s838, public;
COMMIT;
-- batch 105
SELECT id, name FROM accounts WHERE id = 840;
INSERT INTO events (id, kind) VALUES (841, 'k841');
UPDATE accounts SET balance = balance + 842 WHERE id = 842;
DELETE FROM sessions WHERE expires < 843;
CREATE TABLE t844 (id int PRIMARY KEY, v text);
GRANT SELECT ON t845 TO reader;
SET search_path TO s846, public;
COMMIT;
-- batch 106
SELECT id, name FROM accounts WHERE id = 848;
INSERT INTO events (id, kind) VALUES (849, 'k849');
UPDATE accounts SET balance = balance + 850 WHERE id = 850;
DELETE FROM sessions WHERE expires < 851;
CREATE TABLE t852 (id int PRIMARY KEY, v text);
GRANT SELECT ON t853 TO reader;
SET search_path TO s854, public;
COMMIT;
-- batch 107
SELECT id, name FROM accounts WHERE id = 856;
INSERT INTO events (id, kind) VALUES (857, 'k857');
UPDATE accounts SET balance = balance + 858 WHERE id = 858;
DELETE FROM sessions WHERE expires < 859;
CREATE TABLE t860 (id int PRIMARY KEY, v text);
GRANT SELECT ON t861 TO reader;
SET search_path TO s862, public;
COMMIT;
-- batch 108
SELECT id, name FROM accounts WHERE id = 864;
INSERT INTO events (id, kind) VALUES (865, 'k865');
UPDATE accounts SET balance = balance + 866 WHERE id = 866;
DELETE FROM sessions WHERE expires < 867;
CREATE TABLE t868 (id int PRIMARY KEY, v text);
GRANT SELECT ON t869 TO reader;
SET search_path TO s870, public;
COMMIT;
-- batch 109
SELECT id, name FROM accounts WHERE id = 872;
INSERT INTO events (id, kind) VALUES (873, 'k873');
UPDATE accounts SET balance = balance + 874 WHERE id = 874;
DELETE FROM sessions WHERE expires < 875;
CREATE TABLE t876 (id int PRIMARY KEY, v text);
GRANT SELECT ON t877 TO reader;
SET search_path TO s878, public;
COMMIT;
-- batch 110
SELECT id, name FROM accounts WHERE id = 880;
INSERT INTO events (id, kind) VALUES (881, 'k881');
UPDATE accounts SET balance = balance + 882 WHERE id = 882;
DELETE FROM sessions WHERE expires < 883;
CREATE TABLE t884 (id int PRIMARY KEY, v text);
GRANT SELECT ON t885 TO reader;
SET search_path TO s886, public;
COMMIT;
-- batch 111
SELECT id, name FROM accounts WHERE id = 888;
INSERT INTO events (id, kind) VALUES (889, 'k889');
UPDATE accounts SET balance = balance + 890 WHERE id = 890;
DELETE FROM sessions WHERE expires < 891;
CREATE TABLE t892 (id int PRIMARY KEY, v text);
GRANT SELECT ON t893 TO reader;
SET search_path TO s894, public;
COMMIT;
-- batch 112
SELECT id, name FROM accounts WHERE id = 896;
INSERT INTO events (id, kind) VALUES (897, 'k897');
UPDATE accounts SET balance = balance + 898 WHERE id = 898;
DELETE FROM sessions WHERE expires < 899;
CREATE TABLE t900 (id int PRIMARY KEY, v text);
GRANT SELECT ON t901 TO reader;
SET search_path TO s902, public;
COMMIT;
-- batch 113
SELECT id, name FROM accounts WHERE id = 904;
INSERT INTO events (id, kind) VALUES (905, 'k905');
UPDATE accounts SET balance = balance + 906 WHERE id = 906;
DELETE FROM sessions WHERE expires < 907;
CREATE TABLE t908 (id int PRIMARY KEY, v text);
GRANT SELECT ON t909 TO reader;
SET search_path TO s910, public;
COMMIT;
-- batch 114
SELECT id, name FROM accounts WHERE id = 912;
INSERT INTO events (id, kind) VALUES (913, 'k913');
UPDATE accounts SET balance = balance + 914 WHERE id = 914;
DELETE FROM sessions WHERE expires < 915;
CREATE TABLE t916 (id int PRIMARY KEY, v text);
GRANT SELECT ON t917 TO reader;
SET search_path TO s918, public;
COMMIT;
-- batch 115
SELECT id, name FROM accounts WHERE id = 920;
INSERT INTO events (id, kind) VALUES (921, 'k921');
UPDATE accounts SET balance = balance + 922 WHERE id = 922;
DELETE FROM sessions WHERE expires < 923;
CREATE TABLE t924 (id int PRIMARY KEY, v text);
GRANT SELECT ON t925 TO reader;
SET search_path TO s926, public;
COMMIT;
-- batch 116
SELECT id, name FROM accounts WHERE id = 928;
INSERT INTO events (id, kind) VALUES (929, 'k929');
UPDATE accounts SET balance = balance + 930 WHERE id = 930;
DELETE FROM sessions WHERE expires < 931;
CREATE TABLE t932 (id int PRIMARY KEY, v text);
GRANT SELECT ON t933 TO reader;
SET search_path TO s934, public;
COMMIT;
-- batch 117
SELECT id, name FROM accounts WHERE id = 936;
INSERT INTO events (id, kind) VALUES (937, 'k937');
UPDATE accounts SET balance = balance + 938 WHERE id = 938;
DELETE FROM sessions WHERE expires < 939;
CREATE TABLE t940 (id int PRIMARY KEY, v text);
GRANT SELECT ON t941 TO reader;
SET search_path TO s942, public;
COMMIT;
-- batch 118
SELECT id, name FROM accounts WHERE id = 944;
INSERT INTO events (id, kind) VALUES (945, 'k945');
UPDATE accounts SET balance = balance + 946 WHERE id = 946;
DELETE FROM sessions WHERE expires < 947;
CREATE TABLE t948 (id int PRIMARY KEY, v text);
GRANT SELECT ON t949 TO reader;
SET search_path TO s950, public;
COMMIT;
-- batch 119
SELECT id, name FROM accounts WHERE id = 952;
INSERT INTO events (id, kind) VALUES (953, 'k953');
UPDATE accounts SET balance = balance + 954 WHERE id = 954;
DELETE FROM sessions WHERE expires < 955;
CREATE TABLE t956 (id int PRIMARY KEY, v text);
GRANT SELECT ON t957 TO reader;
SET search_path TO s958, public;
COMMIT;
-- batch 120
SELECT id, name FROM accounts WHERE id = 960;
INSERT INTO events (id, kind) VALUES (961, 'k961');
UPDATE accounts SET balance = balance + 962 WHERE id = 962;
DELETE FROM sessions WHERE expires < 963;
CREATE TABLE t964 (id int PRIMARY KEY, v text);
GRANT SELECT ON t965 TO reader;
SET search_path TO s966, public;
COMMIT;
-- batch 121
SELECT id, name FROM accounts WHERE id = 968;
INSERT INTO events (id, kind) VALUES (969, 'k969');
UPDATE accounts SET balance = balance + 970 WHERE id = 970;
DELETE FROM sessions WHERE expires < 971;
CREATE TABLE t972 (id int PRIMARY KEY, v text);
GRANT SELECT ON t973 TO reader;
SET search_path TO s974, public;
COMMIT;
-- batch 122
SELECT id, name FROM accounts WHERE id = 976;
INSERT INTO events (id, kind) VALUES (977, 'k977');
UPDATE accounts SET balance = balance + 978 WHERE id = 978;
DELETE FROM sessions WHERE expires < 979;
CREATE TABLE t980 (id int PRIMARY KEY, v text);
GRANT SELECT ON t981 TO reader;
SET search_path TO s982, public;
COMMIT;
-- batch 123
SELECT id, name FROM accounts WHERE id = 984;
INSERT INTO events (id, kind) VALUES (985, 'k985');
UPDATE accounts SET balance = balance + 986 WHERE id = 986;
DELETE FROM sessions WHERE expires < 987;
CREATE TABLE t988 (id int PRIMARY KEY, v text);
GRANT SELECT ON t989 TO reader;
SET search_path TO s990, public;
COMMIT;
-- batch 124
SELECT id, name FROM accounts WHERE id = 992;
INSERT INTO events (id, kind) VALUES (993, 'k993');
UPDATE accounts SET balance = balance + 994 WHERE id = 994;
DELETE FROM sessions WHERE expires < 995;
CREATE TABLE t996 (id int PRIMARY KEY, v text);
GRANT SELECT ON t997 TO reader;
SET search_path TO s998, public;
COMMIT;
-- batch 125
SELECT id, name FROM accounts WHERE id = 1000;
INSERT INTO events (id, kind) VALUES (1001, 'k1001');
UPDATE accounts SET balance = balance + 1002 WHERE id = 1002;
DELETE FROM sessions WHERE expires < 1003;
CREATE TABLE t1004 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1005 TO reader;
SET search_path TO s1006, public;
COMMIT;
-- batch 126
SELECT id, name FROM accounts WHERE id = 1008;
INSERT INTO events (id, kind) VALUES (1009, 'k1009');
UPDATE accounts SET balance = balance + 1010 WHERE id = 1010;
DELETE FROM sessions WHERE expires < 1011;
CREATE TABLE t1012 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1013 TO reader;
SET search_path TO s1014, public;
COMMIT;
-- batch 127
SELECT id, name FROM accounts WHERE id = 1016;
INSERT INTO events (id, kind) VALUES (1017, 'k1017');
UPDATE accounts SET balance = balance + 1018 WHERE id = 1018;
DELETE FROM sessions WHERE expires < 1019;
CREATE TABLE t1020 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1021 TO reader;
SET search_path TO s1022, public;
COMMIT;
-- batch 128
SELECT id, name FROM accounts WHERE id = 1024;
INSERT INTO events (id, kind) VALUES (1025, 'k1025');
UPDATE accounts SET balance = balance + 1026 WHERE id = 1026;
DELETE FROM sessions WHERE expires < 1027;
CREATE TABLE t1028 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1029 TO reader;
SET search_path TO s1030, public;
COMMIT;
-- batch 129
SELECT id, name FROM accounts WHERE id = 1032;
INSERT INTO events (id, kind) VALUES (1033, 'k1033');
UPDATE accounts SET balance = balance + 1034 WHERE id = 1034;
DELETE FROM sessions WHERE expires < 1035;
CREATE TABLE t1036 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1037 TO reader;
SET search_path TO s1038, public;
COMMIT;
-- batch 130
SELECT id, name FROM accounts WHERE id = 1040;
INSERT INTO events (id, kind) VALUES (1041, 'k1041');
UPDATE accounts SET balance = balance + 1042 WHERE id = 1042;
DELETE FROM sessions WHERE expires < 1043;
CREATE TABLE t1044 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1045 TO reader;
SET search_path TO s1046, public;
COMMIT;
-- batch 131
SELECT id, name FROM accounts WHERE id = 1048;
INSERT INTO events (id, kind) VALUES (1049, 'k1049');
UPDATE accounts SET balance = balance + 1050 WHERE id = 1050;
DELETE FROM sessions WHERE expires < 1051;
CREATE TABLE t1052 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1053 TO reader;
SET search_path TO s1054, public;
COMMIT;
-- batch 132
SELECT id, name FROM accounts WHERE id = 1056;
INSERT INTO events (id, kind) VALUES (1057, 'k1057');
UPDATE accounts SET balance = balance + 1058 WHERE id = 1058;
DELETE FROM sessions WHERE expires < 1059;
CREATE TABLE t1060 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1061 TO reader;
SET search_path TO s1062, public;
COMMIT;
-- batch 133
SELECT id, name FROM accounts WHERE id = 1064;
INSERT INTO events (id, kind) VALUES (1065, 'k1065');
UPDATE accounts SET balance = balance + 1066 WHERE id = 1066;
DELETE FROM sessions WHERE expires < 1067;
CREATE TABLE t1068 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1069 TO reader;
SET search_path TO s1070, public;
COMMIT;
-- batch 134
SELECT id, name FROM accounts WHERE id = 1072;
INSERT INTO events (id, kind) VALUES (1073, 'k1073');
UPDATE accounts SET balance = balance + 1074 WHERE id = 1074;
DELETE FROM sessions WHERE expires < 1075;
CREATE TABLE t1076 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1077 TO reader;
SET search_path TO s1078, public;
COMMIT;
-- batch 135
SELECT id, name FROM accounts WHERE id = 1080;
INSERT INTO events (id, kind) VALUES (1081, 'k1081');
UPDATE accounts SET balance = balance + 1082 WHERE id = 1082;
DELETE FROM sessions WHERE expires < 1083;
CREATE TABLE t1084 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1085 TO reader;
SET search_path TO s1086, public;
COMMIT;
-- batch 136
SELECT id, name FROM accounts WHERE id = 1088;
INSERT INTO events (id, kind) VALUES (1089, 'k1089');
UPDATE accounts SET balance = balance + 1090 WHERE id = 1090;
DELETE FROM sessions WHERE expires < 1091;
CREATE TABLE t1092 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1093 TO reader;
SET search_path TO s1094, public;
COMMIT;
-- batch 137
SELECT id, name FROM accounts WHERE id = 1096;
INSERT INTO events (id, kind) VALUES (1097, 'k1097');
UPDATE accounts SET balance = balance + 1098 WHERE id = 1098;
DELETE FROM sessions WHERE expires < 1099;
CREATE TABLE t1100 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1101 TO reader;
SET search_path TO s1102, public;
COMMIT;
-- batch 138
SELECT id, name FROM accounts WHERE id = 1104;
INSERT INTO events (id, kind) VALUES (1105, 'k1105');
UPDATE accounts SET balance = balance + 1106 WHERE id = 1106;
DELETE FROM sessions WHERE expires < 1107;
CREATE TABLE t1108 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1109 TO reader;
SET search_path TO s1110, public;
COMMIT;
-- batch 139
SELECT id, name FROM accounts WHERE id = 1112;
INSERT INTO events (id, kind) VALUES (1113, 'k1113');
UPDATE accounts SET balance = balance + 1114 WHERE id = 1114;
DELETE FROM sessions WHERE expires < 1115;
CREATE TABLE t1116 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1117 TO reader;
SET search_path TO s1118, public;
COMMIT;
-- batch 140
SELECT id, name FROM accounts WHERE id = 1120;
INSERT INTO events (id, kind) VALUES (1121, 'k1121');
UPDATE accounts SET balance = balance + 1122 WHERE id = 1122;
DELETE FROM sessions WHERE expires < 1123;
CREATE TABLE t1124 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1125 TO reader;
SET search_path TO s1126, public;
COMMIT;
-- batch 141
SELECT id, name FROM accounts WHERE id = 1128;
INSERT INTO events (id, kind) VALUES (1129, 'k1129');
UPDATE accounts SET balance = balance + 1130 WHERE id = 1130;
DELETE FROM sessions WHERE expires < 1131;
CREATE TABLE t1132 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1133 TO reader;
SET search_path TO s1134, public;
COMMIT;
-- batch 142
SELECT id, name FROM accounts WHERE id = 1136;
INSERT INTO events (id, kind) VALUES (1137, 'k1137');
UPDATE accounts SET balance = balance + 1138 WHERE id = 1138;
DELETE FROM sessions WHERE expires < 1139;
CREATE TABLE t1140 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1141 TO reader;
SET search_path TO s1142, public;
COMMIT;
-- batch 143
SELECT id, name FROM accounts WHERE id = 1144;
INSERT INTO events (id, kind) VALUES (1145, 'k1145');
UPDATE accounts SET balance = balance + 1146 WHERE id = 1146;
DELETE FROM sessions WHERE expires < 1147;
CREATE TABLE t1148 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1149 TO reader;
SET search_path TO s1150, public;
COMMIT;
-- batch 144
SELECT id, name FROM accounts WHERE id = 1152;
INSERT INTO events (id, kind) VALUES (1153, 'k1153');
UPDATE accounts SET balance = balance + 1154 WHERE id = 1154;
DELETE FROM sessions WHERE expires < 1155;
CREATE TABLE t1156 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1157 TO reader;
SET search_path TO s1158, public;
COMMIT;
-- batch 145
SELECT id, name FROM accounts WHERE id = 1160;
INSERT INTO events (id, kind) VALUES (1161, 'k1161');
UPDATE accounts SET balance = balance + 1162 WHERE id = 1162;
DELETE FROM sessions WHERE expires < 1163;
CREATE TABLE t1164 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1165 TO reader;
SET search_path TO s1166, public;
COMMIT;
-- batch 146
SELECT id, name FROM accounts WHERE id = 1168;
INSERT INTO events (id, kind) VALUES (1169, 'k1169');
UPDATE accounts SET balance = balance + 1170 WHERE id = 1170;
DELETE FROM sessions WHERE expires < 1171;
CREATE TABLE t1172 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1173 TO reader;
SET search_path TO s1174, public;
COMMIT;
-- batch 147
SELECT id, name FROM accounts WHERE id = 1176;
INSERT INTO events (id, kind) VALUES (1177, 'k1177');
UPDATE accounts SET balance = balance + 1178 WHERE id = 1178;
DELETE FROM sessions WHERE expires < 1179;
CREATE TABLE t1180 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1181 TO reader;
SET search_path TO s1182, public;
COMMIT;
-- batch 148
SELECT id, name FROM accounts WHERE id = 1184;
INSERT INTO events (id, kind) VALUES (1185, 'k1185');
UPDATE accounts SET balance = balance + 1186 WHERE id = 1186;
DELETE FROM sessions WHERE expires < 1187;
CREATE TABLE t1188 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1189 TO reader;
SET search_path TO s1190, public;
COMMIT;
-- batch 149
SELECT id, name FROM accounts WHERE id = 1192;
INSERT INTO events (id, kind) VALUES (1193, 'k1193');
UPDATE accounts SET balance = balance + 1194 WHERE id = 1194;
DELETE FROM sessions WHERE expires < 1195;
CREATE TABLE t1196 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1197 TO reader;
SET search_path TO s1198, public;
COMMIT;
-- batch 150
SELECT id, name FROM accounts WHERE id = 1200;
INSERT INTO events (id, kind) VALUES (1201, 'k1201');
UPDATE accounts SET balance = balance + 1202 WHERE id = 1202;
DELETE FROM sessions WHERE expires < 1203;
CREATE TABLE t1204 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1205 TO reader;
SET search_path TO s1206, public;
COMMIT;
-- batch 151
SELECT id, name FROM accounts WHERE id = 1208;
INSERT INTO events (id, kind) VALUES (1209, 'k1209');
UPDATE accounts SET balance = balance + 1210 WHERE id = 1210;
DELETE FROM sessions WHERE expires < 1211;
CREATE TABLE t1212 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1213 TO reader;
SET search_path TO s1214, public;
COMMIT;
-- batch 152
SELECT id, name FROM accounts WHERE id = 1216;
INSERT INTO events (id, kind) VALUES (1217, 'k1217');
UPDATE accounts SET balance = balance + 1218 WHERE id = 1218;
DELETE FROM sessions WHERE expires < 1219;
CREATE TABLE t1220 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1221 TO reader;
SET search_path TO s1222, public;
COMMIT;
-- batch 153
SELECT id, name FROM accounts WHERE id = 1224;
INSERT INTO events (id, kind) VALUES (1225, 'k1225');
UPDATE accounts SET balance = balance + 1226 WHERE id = 1226;
DELETE FROM sessions WHERE expires < 1227;
CREATE TABLE t1228 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1229 TO reader;
SET search_path TO s1230, public;
COMMIT;
-- batch 154
SELECT id, name FROM accounts WHERE id = 1232;
INSERT INTO events (id, kind) VALUES (1233, 'k1233');
UPDATE accounts SET balance = balance + 1234 WHERE id = 1234;
DELETE FROM sessions WHERE expires < 1235;
CREATE TABLE t1236 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1237 TO reader;
SET search_path TO s1238, public;
COMMIT;
-- batch 155
SELECT id, name FROM accounts WHERE id = 1240;
INSERT INTO events (id, kind) VALUES (1241, 'k1241');
UPDATE accounts SET balance = balance + 1242 WHERE id = 1242;
DELETE FROM sessions WHERE expires < 1243;
CREATE TABLE t1244 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1245 TO reader;
SET search_path TO s1246, public;
COMMIT;
-- batch 156
SELECT id, name FROM accounts WHERE id = 1248;
INSERT INTO events (id, kind) VALUES (1249, 'k1249');
UPDATE accounts SET balance = balance + 1250 WHERE id = 1250;
DELETE FROM sessions WHERE expires < 1251;
CREATE TABLE t1252 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1253 TO reader;
SET search_path TO s1254, public;
COMMIT;
-- batch 157
SELECT id, name FROM accounts WHERE id = 1256;
INSERT INTO events (id, kind) VALUES (1257, 'k1257');
UPDATE accounts SET balance = balance + 1258 WHERE id = 1258;
DELETE FROM sessions WHERE expires < 1259;
CREATE TABLE t1260 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1261 TO reader;
SET search_path TO s1262, public;
COMMIT;
-- batch 158
SELECT id, name FROM accounts WHERE id = 1264;
INSERT INTO events (id, kind) VALUES (1265, 'k1265');
UPDATE accounts SET balance = balance + 1266 WHERE id = 1266;
DELETE FROM sessions WHERE expires < 1267;
CREATE TABLE t1268 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1269 TO reader;
SET search_path TO s1270, public;
COMMIT;
-- batch 159
SELECT id, name FROM accounts WHERE id = 1272;
INSERT INTO events (id, kind) VALUES (1273, 'k1273');
UPDATE accounts SET balance = balance + 1274 WHERE id = 1274;
DELETE FROM sessions WHERE expires < 1275;
CREATE TABLE t1276 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1277 TO reader;
SET search_path TO s1278, public;
COMMIT;
-- batch 160
SELECT id, name FROM accounts WHERE id = 1280;
INSERT INTO events (id, kind) VALUES (1281, 'k1281');
UPDATE accounts SET balance = balance + 1282 WHERE id = 1282;
DELETE FROM sessions WHERE expires < 1283;
CREATE TABLE t1284 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1285 TO reader;
SET search_path TO s1286, public;
COMMIT;
-- batch 161
SELECT id, name FROM accounts WHERE id = 1288;
INSERT INTO events (id, kind) VALUES (1289, 'k1289');
UPDATE accounts SET balance = balance + 1290 WHERE id = 1290;
DELETE FROM sessions WHERE expires < 1291;
CREATE TABLE t1292 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1293 TO reader;
SET search_path TO s1294, public;
COMMIT;
-- batch 162
SELECT id, name FROM accounts WHERE id = 1296;
INSERT INTO events (id, kind) VALUES (1297, 'k1297');
UPDATE accounts SET balance = balance + 1298 WHERE id = 1298;
DELETE FROM sessions WHERE expires < 1299;
CREATE TABLE t1300 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1301 TO reader;
SET search_path TO s1302, public;
COMMIT;
-- batch 163
SELECT id, name FROM accounts WHERE id = 1304;
INSERT INTO events (id, kind) VALUES (1305, 'k1305');
UPDATE accounts SET balance = balance + 1306 WHERE id = 1306;
DELETE FROM sessions WHERE expires < 1307;
CREATE TABLE t1308 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1309 TO reader;
SET search_path TO s1310, public;
COMMIT;
-- batch 164
SELECT id, name FROM accounts WHERE id = 1312;
INSERT INTO events (id, kind) VALUES (1313, 'k1313');
UPDATE accounts SET balance = balance + 1314 WHERE id = 1314;
DELETE FROM sessions WHERE expires < 1315;
CREATE TABLE t1316 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1317 TO reader;
SET search_path TO s1318, public;
COMMIT;
-- batch 165
SELECT id, name FROM accounts WHERE id = 1320;
INSERT INTO events (id, kind) VALUES (1321, 'k1321');
UPDATE accounts SET balance = balance + 1322 WHERE id = 1322;
DELETE FROM sessions WHERE expires < 1323;
CREATE TABLE t1324 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1325 TO reader;
SET search_path TO s1326, public;
COMMIT;
-- batch 166
SELECT id, name FROM accounts WHERE id = 1328;
INSERT INTO events (id, kind) VALUES (1329, 'k1329');
UPDATE accounts SET balance = balance + 1330 WHERE id = 1330;
DELETE FROM sessions WHERE expires < 1331;
CREATE TABLE t1332 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1333 TO reader;
SET search_path TO s1334, public;
COMMIT;
-- batch 167
SELECT id, name FROM accounts WHERE id = 1336;
INSERT INTO events (id, kind) VALUES (1337, 'k1337');
UPDATE accounts SET balance = balance + 1338 WHERE id = 1338;
DELETE FROM sessions WHERE expires < 1339;
CREATE TABLE t1340 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1341 TO reader;
SET search_path TO s1342, public;
COMMIT;
-- batch 168
SELECT id, name FROM accounts WHERE id = 1344;
INSERT INTO events (id, kind) VALUES (1345, 'k1345');
UPDATE accounts SET balance = balance + 1346 WHERE id = 1346;
DELETE FROM sessions WHERE expires < 1347;
CREATE TABLE t1348 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1349 TO reader;
SET search_path TO s1350, public;
COMMIT;
-- batch 169
SELECT id, name FROM accounts WHERE id = 1352;
INSERT INTO events (id, kind) VALUES (1353, 'k1353');
UPDATE accounts SET balance = balance + 1354 WHERE id = 1354;
DELETE FROM sessions WHERE expires < 1355;
CREATE TABLE t1356 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1357 TO reader;
SET search_path TO s1358, public;
COMMIT;
-- batch 170
SELECT id, name FROM accounts WHERE id = 1360;
INSERT INTO events (id, kind) VALUES (1361, 'k1361');
UPDATE accounts SET balance = balance + 1362 WHERE id = 1362;
DELETE FROM sessions WHERE expires < 1363;
CREATE TABLE t1364 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1365 TO reader;
SET search_path TO s1366, public;
COMMIT;
-- batch 171
SELECT id, name FROM accounts WHERE id = 1368;
INSERT INTO events (id, kind) VALUES (1369, 'k1369');
UPDATE accounts SET balance = balance + 1370 WHERE id = 1370;
DELETE FROM sessions WHERE expires < 1371;
CREATE TABLE t1372 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1373 TO reader;
SET search_path TO s1374, public;
COMMIT;
-- batch 172
SELECT id, name FROM accounts WHERE id = 1376;
INSERT INTO events (id, kind) VALUES (1377, 'k1377');
UPDATE accounts SET balance = balance + 1378 WHERE id = 1378;
DELETE FROM sessions WHERE expires < 1379;
CREATE TABLE t1380 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1381 TO reader;
SET search_path TO s1382, public;
COMMIT;
-- batch 173
SELECT id, name FROM accounts WHERE id = 1384;
INSERT INTO events (id, kind) VALUES (1385, 'k1385');
UPDATE accounts SET balance = balance + 1386 WHERE id = 1386;
DELETE FROM sessions WHERE expires < 1387;
CREATE TABLE t1388 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1389 TO reader;
SET search_path TO s1390, public;
COMMIT;
-- batch 174
SELECT id, name FROM accounts WHERE id = 1392;
INSERT INTO events (id, kind) VALUES (1393, 'k1393');
UPDATE accounts SET balance = balance + 1394 WHERE id = 1394;
DELETE FROM sessions WHERE expires < 1395;
CREATE TABLE t1396 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1397 TO reader;
SET search_path TO s1398, public;
COMMIT;
-- batch 175
SELECT id, name FROM accounts WHERE id = 1400;
INSERT INTO events (id, kind) VALUES (1401, 'k1401');
UPDATE accounts SET balance = balance + 1402 WHERE id = 1402;
DELETE FROM sessions WHERE expires < 1403;
CREATE TABLE t1404 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1405 TO reader;
SET search_path TO s1406, public;
COMMIT;
-- batch 176
SELECT id, name FROM accounts WHERE id = 1408;
INSERT INTO events (id, kind) VALUES (1409, 'k1409');
UPDATE accounts SET balance = balance + 1410 WHERE id = 1410;
DELETE FROM sessions WHERE expires < 1411;
CREATE TABLE t1412 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1413 TO reader;
SET search_path TO s1414, public;
COMMIT;
-- batch 177
SELECT id, name FROM accounts WHERE id = 1416;
INSERT INTO events (id, kind) VALUES (1417, 'k1417');
UPDATE accounts SET balance = balance + 1418 WHERE id = 1418;
DELETE FROM sessions WHERE expires < 1419;
CREATE TABLE t1420 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1421 TO reader;
SET search_path TO s1422, public;
COMMIT;
-- batch 178
SELECT id, name FROM accounts WHERE id = 1424;
INSERT INTO events (id, kind) VALUES (1425, 'k1425');
UPDATE accounts SET balance = balance + 1426 WHERE id = 1426;
DELETE FROM sessions WHERE expires < 1427;
CREATE TABLE t1428 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1429 TO reader;
SET search_path TO s1430, public;
COMMIT;
-- batch 179
SELECT id, name FROM accounts WHERE id = 1432;
INSERT INTO events (id, kind) VALUES (1433, 'k1433');
UPDATE accounts SET balance = balance + 1434 WHERE id = 1434;
DELETE FROM sessions WHERE expires < 1435;
CREATE TABLE t1436 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1437 TO reader;
SET search_path TO s1438, public;
COMMIT;
-- batch 180
SELECT id, name FROM accounts WHERE id = 1440;
INSERT INTO events (id, kind) VALUES (1441, 'k1441');
UPDATE accounts SET balance = balance + 1442 WHERE id = 1442;
DELETE FROM sessions WHERE expires < 1443;
CREATE TABLE t1444 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1445 TO reader;
SET search_path TO s1446, public;
COMMIT;
-- batch 181
SELECT id, name FROM accounts WHERE id = 1448;
INSERT INTO events (id, kind) VALUES (1449, 'k1449');
UPDATE accounts SET balance = balance + 1450 WHERE id = 1450;
DELETE FROM sessions WHERE expires < 1451;
CREATE TABLE t1452 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1453 TO reader;
SET search_path TO s1454, public;
COMMIT;
-- batch 182
SELECT id, name FROM accounts WHERE id = 1456;
INSERT INTO events (id, kind) VALUES (1457, 'k1457');
UPDATE accounts SET balance = balance + 1458 WHERE id = 1458;
DELETE FROM sessions WHERE expires < 1459;
CREATE TABLE t1460 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1461 TO reader;
SET search_path TO s1462, public;
COMMIT;
-- batch 183
SELECT id, name FROM accounts WHERE id = 1464;
INSERT INTO events (id, kind) VALUES (1465, 'k1465');
UPDATE accounts SET balance = balance + 1466 WHERE id = 1466;
DELETE FROM sessions WHERE expires < 1467;
CREATE TABLE t1468 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1469 TO reader;
SET search_path TO s1470, public;
COMMIT;
-- batch 184
SELECT id, name FROM accounts WHERE id = 1472;
INSERT INTO events (id, kind) VALUES (1473, 'k1473');
UPDATE accounts SET balance = balance + 1474 WHERE id = 1474;
DELETE FROM sessions WHERE expires < 1475;
CREATE TABLE t1476 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1477 TO reader;
SET search_path TO s1478, public;
COMMIT;
-- batch 185
SELECT id, name FROM accounts WHERE id = 1480;
INSERT INTO events (id, kind) VALUES (1481, 'k1481');
UPDATE accounts SET balance = balance + 1482 WHERE id = 1482;
DELETE FROM sessions WHERE expires < 1483;
CREATE TABLE t1484 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1485 TO reader;
SET search_path TO s1486, public;
COMMIT;
-- batch 186
SELECT id, name FROM accounts WHERE id = 1488;
INSERT INTO events (id, kind) VALUES (1489, 'k1489');
UPDATE accounts SET balance = balance + 1490 WHERE id = 1490;
DELETE FROM sessions WHERE expires < 1491;
CREATE TABLE t1492 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1493 TO reader;
SET search_path TO s1494, public;
COMMIT;
-- batch 187
SELECT id, name FROM accounts WHERE id = 1496;
INSERT INTO events (id, kind) VALUES (1497, 'k1497');
UPDATE accounts SET balance = balance + 1498 WHERE id = 1498;
DELETE FROM sessions WHERE expires < 1499;
CREATE TABLE t1500 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1501 TO reader;
SET search_path TO s1502, public;
COMMIT;
-- batch 188
SELECT id, name FROM accounts WHERE id = 1504;
INSERT INTO events (id, kind) VALUES (1505, 'k1505');
UPDATE accounts SET balance = balance + 1506 WHERE id = 1506;
DELETE FROM sessions WHERE expires < 1507;
CREATE TABLE t1508 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1509 TO reader;
SET search_path TO s1510, public;
COMMIT;
-- batch 189
SELECT id, name FROM accounts WHERE id = 1512;
INSERT INTO events (id, kind) VALUES (1513, 'k1513');
UPDATE accounts SET balance = balance + 1514 WHERE id = 1514;
DELETE FROM sessions WHERE expires < 1515;
CREATE TABLE t1516 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1517 TO reader;
SET search_path TO s1518, public;
COMMIT;
-- batch 190
SELECT id, name FROM accounts WHERE id = 1520;
INSERT INTO events (id, kind) VALUES (1521, 'k1521');
UPDATE accounts SET balance = balance + 1522 WHERE id = 1522;
DELETE FROM sessions WHERE expires < 1523;
CREATE TABLE t1524 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1525 TO reader;
SET search_path TO s1526, public;
COMMIT;
-- batch 191
SELECT id, name FROM accounts WHERE id = 1528;
INSERT INTO events (id, kind) VALUES (1529, 'k1529');
UPDATE accounts SET balance = balance + 1530 WHERE id = 1530;
DELETE FROM sessions WHERE expires < 1531;
CREATE TABLE t1532 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1533 TO reader;
SET search_path TO s1534, public;
COMMIT;
-- batch 192
SELECT id, name FROM accounts WHERE id = 1536;
INSERT INTO events (id, kind) VALUES (1537, 'k1537');
UPDATE accounts SET balance = balance + 1538 WHERE id = 1538;
DELETE FROM sessions WHERE expires < 1539;
CREATE TABLE t1540 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1541 TO reader;
SET search_path TO s1542, public;
COMMIT;
-- batch 193
SELECT id, name FROM accounts WHERE id = 1544;
INSERT INTO events (id, kind) VALUES (1545, 'k1545');
UPDATE accounts SET balance = balance + 1546 WHERE id = 1546;
DELETE FROM sessions WHERE expires < 1547;
CREATE TABLE t1548 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1549 TO reader;
SET search_path TO s1550, public;
COMMIT;
-- batch 194
SELECT id, name FROM accounts WHERE id = 1552;
INSERT INTO events (id, kind) VALUES (1553, 'k1553');
UPDATE accounts SET balance = balance + 1554 WHERE id = 1554;
DELETE FROM sessions WHERE expires < 1555;
CREATE TABLE t1556 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1557 TO reader;
SET search_path TO s1558, public;
COMMIT;
-- batch 195
SELECT id, name FROM accounts WHERE id = 1560;
INSERT INTO events (id, kind) VALUES (1561, 'k1561');
UPDATE accounts SET balance = balance + 1562 WHERE id = 1562;
DELETE FROM sessions WHERE expires < 1563;
CREATE TABLE t1564 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1565 TO reader;
SET search_path TO s1566, public;
COMMIT;
-- batch 196
SELECT id, name FROM accounts WHERE id = 1568;
INSERT INTO events (id, kind) VALUES (1569, 'k1569');
UPDATE accounts SET balance = balance + 1570 WHERE id = 1570;
DELETE FROM sessions WHERE expires < 1571;
CREATE TABLE t1572 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1573 TO reader;
SET search_path TO s1574, public;
COMMIT;
-- batch 197
SELECT id, name FROM accounts WHERE id = 1576;
INSERT INTO events (id, kind) VALUES (1577, 'k1577');
UPDATE accounts SET balance = balance + 1578 WHERE id = 1578;
DELETE FROM sessions WHERE expires < 1579;
CREATE TABLE t1580 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1581 TO reader;
SET search_path TO s1582, public;
COMMIT;
-- batch 198
SELECT id, name FROM accounts WHERE id = 1584;
INSERT INTO events (id, kind) VALUES (1585, 'k1585');
UPDATE accounts SET balance = balance + 1586 WHERE id = 1586;
DELETE FROM sessions WHERE expires < 1587;
CREATE TABLE t1588 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1589 TO reader;
SET search_path TO s1590, public;
COMMIT;
-- batch 199
SELECT id, name FROM accounts WHERE id = 1592;
INSERT INTO events (id, kind) VALUES (1593, 'k1593');
UPDATE accounts SET balance = balance + 1594 WHERE id = 1594;
DELETE FROM sessions WHERE expires < 1595;
CREATE TABLE t1596 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1597 TO reader;
SET search_path TO s1598, public;
COMMIT;
-- batch 200
SELECT id, name FROM accounts WHERE id = 1600;
INSERT INTO events (id, kind) VALUES (1601, 'k1601');
UPDATE accounts SET balance = balance + 1602 WHERE id = 1602;
DELETE FROM sessions WHERE expires < 1603;
CREATE TABLE t1604 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1605 TO reader;
SET search_path TO s1606, public;
COMMIT;
-- batch 201
SELECT id, name FROM accounts WHERE id = 1608;
INSERT INTO events (id, kind) VALUES (1609, 'k1609');
UPDATE accounts SET balance = balance + 1610 WHERE id = 1610;
DELETE FROM sessions WHERE expires < 1611;
CREATE TABLE t1612 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1613 TO reader;
SET search_path TO s1614, public;
COMMIT;
-- batch 202
SELECT id, name FROM accounts WHERE id = 1616;
INSERT INTO events (id, kind) VALUES (1617, 'k1617');
UPDATE accounts SET balance = balance + 1618 WHERE id = 1618;
DELETE FROM sessions WHERE expires < 1619;
CREATE TABLE t1620 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1621 TO reader;
SET search_path TO s1622, public;
COMMIT;
-- batch 203
SELECT id, name FROM accounts WHERE id = 1624;
INSERT INTO events (id, kind) VALUES (1625, 'k1625');
UPDATE accounts SET balance = balance + 1626 WHERE id = 1626;
DELETE FROM sessions WHERE expires < 1627;
CREATE TABLE t1628 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1629 TO reader;
SET search_path TO s1630, public;
COMMIT;
-- batch 204
SELECT id, name FROM accounts WHERE id = 1632;
INSERT INTO events (id, kind) VALUES (1633, 'k1633');
UPDATE accounts SET balance = balance + 1634 WHERE id = 1634;
DELETE FROM sessions WHERE expires < 1635;
CREATE TABLE t1636 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1637 TO reader;
SET search_path TO s1638, public;
COMMIT;
-- batch 205
SELECT id, name FROM accounts WHERE id = 1640;
INSERT INTO events (id, kind) VALUES (1641, 'k1641');
UPDATE accounts SET balance = balance + 1642 WHERE id = 1642;
DELETE FROM sessions WHERE expires < 1643;
CREATE TABLE t1644 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1645 TO reader;
SET search_path TO s1646, public;
COMMIT;
-- batch 206
SELECT id, name FROM accounts WHERE id = 1648;
INSERT INTO events (id, kind) VALUES (1649, 'k1649');
UPDATE accounts SET balance = balance + 1650 WHERE id = 1650;
DELETE FROM sessions WHERE expires < 1651;
CREATE TABLE t1652 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1653 TO reader;
SET search_path TO s1654, public;
COMMIT;
-- batch 207
SELECT id, name FROM accounts WHERE id = 1656;
INSERT INTO events (id, kind) VALUES (1657, 'k1657');
UPDATE accounts SET balance = balance + 1658 WHERE id = 1658;
DELETE FROM sessions WHERE expires < 1659;
CREATE TABLE t1660 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1661 TO reader;
SET search_path TO s1662, public;
COMMIT;
-- batch 208
SELECT id, name FROM accounts WHERE id = 1664;
INSERT INTO events (id, kind) VALUES (1665, 'k1665');
UPDATE accounts SET balance = balance + 1666 WHERE id = 1666;
DELETE FROM sessions WHERE expires < 1667;
CREATE TABLE t1668 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1669 TO reader;
SET search_path TO s1670, public;
COMMIT;
-- batch 209
SELECT id, name FROM accounts WHERE id = 1672;
INSERT INTO events (id, kind) VALUES (1673, 'k1673');
UPDATE accounts SET balance = balance + 1674 WHERE id = 1674;
DELETE FROM sessions WHERE expires < 1675;
CREATE TABLE t1676 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1677 TO reader;
SET search_path TO s1678, public;
COMMIT;
-- batch 210
SELECT id, name FROM accounts WHERE id = 1680;
INSERT INTO events (id, kind) VALUES (1681, 'k1681');
UPDATE accounts SET balance = balance + 1682 WHERE id = 1682;
DELETE FROM sessions WHERE expires < 1683;
CREATE TABLE t1684 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1685 TO reader;
SET search_path TO s1686, public;
COMMIT;
-- batch 211
SELECT id, name FROM accounts WHERE id = 1688;
INSERT INTO events (id, kind) VALUES (1689, 'k1689');
UPDATE accounts SET balance = balance + 1690 WHERE id = 1690;
DELETE FROM sessions WHERE expires < 1691;
CREATE TABLE t1692 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1693 TO reader;
SET search_path TO s1694, public;
COMMIT;
-- batch 212
SELECT id, name FROM accounts WHERE id = 1696;
INSERT INTO events (id, kind) VALUES (1697, 'k1697');
UPDATE accounts SET balance = balance + 1698 WHERE id = 1698;
DELETE FROM sessions WHERE expires < 1699;
CREATE TABLE t1700 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1701 TO reader;
SET search_path TO s1702, public;
COMMIT;
-- batch 213
SELECT id, name FROM accounts WHERE id = 1704;
INSERT INTO events (id, kind) VALUES (1705, 'k1705');
UPDATE accounts SET balance = balance + 1706 WHERE id = 1706;
DELETE FROM sessions WHERE expires < 1707;
CREATE TABLE t1708 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1709 TO reader;
SET search_path TO s1710, public;
COMMIT;
-- batch 214
SELECT id, name FROM accounts WHERE id = 1712;
INSERT INTO events (id, kind) VALUES (1713, 'k1713');
UPDATE accounts SET balance = balance + 1714 WHERE id = 1714;
DELETE FROM sessions WHERE expires < 1715;
CREATE TABLE t1716 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1717 TO reader;
SET search_path TO s1718, public;
COMMIT;
-- batch 215
SELECT id, name FROM accounts WHERE id = 1720;
INSERT INTO events (id, kind) VALUES (1721, 'k1721');
UPDATE accounts SET balance = balance + 1722 WHERE id = 1722;
DELETE FROM sessions WHERE expires < 1723;
CREATE TABLE t1724 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1725 TO reader;
SET search_path TO s1726, public;
COMMIT;
-- batch 216
SELECT id, name FROM accounts WHERE id = 1728;
INSERT INTO events (id, kind) VALUES (1729, 'k1729');
UPDATE accounts SET balance = balance + 1730 WHERE id = 1730;
DELETE FROM sessions WHERE expires < 1731;
CREATE TABLE t1732 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1733 TO reader;
SET search_path TO s1734, public;
COMMIT;
-- batch 217
SELECT id, name FROM accounts WHERE id = 1736;
INSERT INTO events (id, kind) VALUES (1737, 'k1737');
UPDATE accounts SET balance = balance + 1738 WHERE id = 1738;
DELETE FROM sessions WHERE expires < 1739;
CREATE TABLE t1740 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1741 TO reader;
SET search_path TO s1742, public;
COMMIT;
-- batch 218
SELECT id, name FROM accounts WHERE id = 1744;
INSERT INTO events (id, kind) VALUES (1745, 'k1745');
UPDATE accounts SET balance = balance + 1746 WHERE id = 1746;
DELETE FROM sessions WHERE expires < 1747;
CREATE TABLE t1748 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1749 TO reader;
SET search_path TO s1750, public;
COMMIT;
-- batch 219
SELECT id, name FROM accounts WHERE id = 1752;
INSERT INTO events (id, kind) VALUES (1753, 'k1753');
UPDATE accounts SET balance = balance + 1754 WHERE id = 1754;
DELETE FROM sessions WHERE expires < 1755;
CREATE TABLE t1756 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1757 TO reader;
SET search_path TO s1758, public;
COMMIT;
-- batch 220
SELECT id, name FROM accounts WHERE id = 1760;
INSERT INTO events (id, kind) VALUES (1761, 'k1761');
UPDATE accounts SET balance = balance + 1762 WHERE id = 1762;
DELETE FROM sessions WHERE expires < 1763;
CREATE TABLE t1764 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1765 TO reader;
SET search_path TO s1766, public;
COMMIT;
-- batch 221
SELECT id, name FROM accounts WHERE id = 1768;
INSERT INTO events (id, kind) VALUES (1769, 'k1769');
UPDATE accounts SET balance = balance + 1770 WHERE id = 1770;
DELETE FROM sessions WHERE expires < 1771;
CREATE TABLE t1772 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1773 TO reader;
SET search_path TO s1774, public;
COMMIT;
-- batch 222
SELECT id, name FROM accounts WHERE id = 1776;
INSERT INTO events (id, kind) VALUES (1777, 'k1777');
UPDATE accounts SET balance = balance + 1778 WHERE id = 1778;
DELETE FROM sessions WHERE expires < 1779;
CREATE TABLE t1780 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1781 TO reader;
SET search_path TO s1782, public;
COMMIT;
-- batch 223
SELECT id, name FROM accounts WHERE id = 1784;
INSERT INTO events (id, kind) VALUES (1785, 'k1785');
UPDATE accounts SET balance = balance + 1786 WHERE id = 1786;
DELETE FROM sessions WHERE expires < 1787;
CREATE TABLE t1788 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1789 TO reader;
SET search_path TO s1790, public;
COMMIT;
-- batch 224
SELECT id, name FROM accounts WHERE id = 1792;
INSERT INTO events (id, kind) VALUES (1793, 'k1793');
UPDATE accounts SET balance = balance + 1794 WHERE id = 1794;
DELETE FROM sessions WHERE expires < 1795;
CREATE TABLE t1796 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1797 TO reader;
SET search_path TO s1798, public;
COMMIT;
-- batch 225
SELECT id, name FROM accounts WHERE id = 1800;
INSERT INTO events (id, kind) VALUES (1801, 'k1801');
UPDATE accounts SET balance = balance + 1802 WHERE id = 1802;
DELETE FROM sessions WHERE expires < 1803;
CREATE TABLE t1804 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1805 TO reader;
SET search_path TO s1806, public;
COMMIT;
-- batch 226
SELECT id, name FROM accounts WHERE id = 1808;
INSERT INTO events (id, kind) VALUES (1809, 'k1809');
UPDATE accounts SET balance = balance + 1810 WHERE id = 1810;
DELETE FROM sessions WHERE expires < 1811;
CREATE TABLE t1812 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1813 TO reader;
SET search_path TO s1814, public;
COMMIT;
-- batch 227
SELECT id, name FROM accounts WHERE id = 1816;
INSERT INTO events (id, kind) VALUES (1817, 'k1817');
UPDATE accounts SET balance = balance + 1818 WHERE id = 1818;
DELETE FROM sessions WHERE expires < 1819;
CREATE TABLE t1820 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1821 TO reader;
SET search_path TO s1822, public;
COMMIT;
-- batch 228
SELECT id, name FROM accounts WHERE id = 1824;
INSERT INTO events (id, kind) VALUES (1825, 'k1825');
UPDATE accounts SET balance = balance + 1826 WHERE id = 1826;
DELETE FROM sessions WHERE expires < 1827;
CREATE TABLE t1828 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1829 TO reader;
SET search_path TO s1830, public;
COMMIT;
-- batch 229
SELECT id, name FROM accounts WHERE id = 1832;
INSERT INTO events (id, kind) VALUES (1833, 'k1833');
UPDATE accounts SET balance = balance + 1834 WHERE id = 1834;
DELETE FROM sessions WHERE expires < 1835;
CREATE TABLE t1836 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1837 TO reader;
SET search_path TO s1838, public;
COMMIT;
-- batch 230
SELECT id, name FROM accounts WHERE id = 1840;
INSERT INTO events (id, kind) VALUES (1841, 'k1841');
UPDATE accounts SET balance = balance + 1842 WHERE id = 1842;
DELETE FROM sessions WHERE expires < 1843;
CREATE TABLE t1844 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1845 TO reader;
SET search_path TO s1846, public;
COMMIT;
-- batch 231
SELECT id, name FROM accounts WHERE id = 1848;
INSERT INTO events (id, kind) VALUES (1849, 'k1849');
UPDATE accounts SET balance = balance + 1850 WHERE id = 1850;
DELETE FROM sessions WHERE expires < 1851;
CREATE TABLE t1852 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1853 TO reader;
SET search_path TO s1854, public;
COMMIT;
-- batch 232
SELECT id, name FROM accounts WHERE id = 1856;
INSERT INTO events (id, kind) VALUES (1857, 'k1857');
UPDATE accounts SET balance = balance + 1858 WHERE id = 1858;
DELETE FROM sessions WHERE expires < 1859;
CREATE TABLE t1860 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1861 TO reader;
SET search_path TO s1862, public;
COMMIT;
-- batch 233
SELECT id, name FROM accounts WHERE id = 1864;
INSERT INTO events (id, kind) VALUES (1865, 'k1865');
UPDATE accounts SET balance = balance + 1866 WHERE id = 1866;
DELETE FROM sessions WHERE expires < 1867;
CREATE TABLE t1868 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1869 TO reader;
SET search_path TO s1870, public;
COMMIT;
-- batch 234
SELECT id, name FROM accounts WHERE id = 1872;
INSERT INTO events (id, kind) VALUES (1873, 'k1873');
UPDATE accounts SET balance = balance + 1874 WHERE id = 1874;
DELETE FROM sessions WHERE expires < 1875;
CREATE TABLE t1876 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1877 TO reader;
SET search_path TO s1878, public;
COMMIT;
-- batch 235
SELECT id, name FROM accounts WHERE id = 1880;
INSERT INTO events (id, kind) VALUES (1881, 'k1881');
UPDATE accounts SET balance = balance + 1882 WHERE id = 1882;
DELETE FROM sessions WHERE expires < 1883;
CREATE TABLE t1884 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1885 TO reader;
SET search_path TO s1886, public;
COMMIT;
-- batch 236
SELECT id, name FROM accounts WHERE id = 1888;
INSERT INTO events (id, kind) VALUES (1889, 'k1889');
UPDATE accounts SET balance = balance + 1890 WHERE id = 1890;
DELETE FROM sessions WHERE expires < 1891;
CREATE TABLE t1892 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1893 TO reader;
SET search_path TO s1894, public;
COMMIT;
-- batch 237
SELECT id, name FROM accounts WHERE id = 1896;
INSERT INTO events (id, kind) VALUES (1897, 'k1897');
UPDATE accounts SET balance = balance + 1898 WHERE id = 1898;
DELETE FROM sessions WHERE expires < 1899;
CREATE TABLE t1900 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1901 TO reader;
SET search_path TO s1902, public;
COMMIT;
-- batch 238
SELECT id, name FROM accounts WHERE id = 1904;
INSERT INTO events (id, kind) VALUES (1905, 'k1905');
UPDATE accounts SET balance = balance + 1906 WHERE id = 1906;
DELETE FROM sessions WHERE expires < 1907;
CREATE TABLE t1908 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1909 TO reader;
SET search_path TO s1910, public;
COMMIT;
-- batch 239
SELECT id, name FROM accounts WHERE id = 1912;
INSERT INTO events (id, kind) VALUES (1913, 'k1913');
UPDATE accounts SET balance = balance + 1914 WHERE id = 1914;
DELETE FROM sessions WHERE expires < 1915;
CREATE TABLE t1916 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1917 TO reader;
SET search_path TO s1918, public;
COMMIT;
-- batch 240
SELECT id, name FROM accounts WHERE id = 1920;
INSERT INTO events (id, kind) VALUES (1921, 'k1921');
UPDATE accounts SET balance = balance + 1922 WHERE id = 1922;
DELETE FROM sessions WHERE expires < 1923;
CREATE TABLE t1924 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1925 TO reader;
SET search_path TO s1926, public;
COMMIT;
-- batch 241
SELECT id, name FROM accounts WHERE id = 1928;
INSERT INTO events (id, kind) VALUES (1929, 'k1929');
UPDATE accounts SET balance = balance + 1930 WHERE id = 1930;
DELETE FROM sessions WHERE expires < 1931;
CREATE TABLE t1932 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1933 TO reader;
SET search_path TO s1934, public;
COMMIT;
-- batch 242
SELECT id, name FROM accounts WHERE id = 1936;
INSERT INTO events (id, kind) VALUES (1937, 'k1937');
UPDATE accounts SET balance = balance + 1938 WHERE id = 1938;
DELETE FROM sessions WHERE expires < 1939;
CREATE TABLE t1940 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1941 TO reader;
SET search_path TO s1942, public;
COMMIT;
-- batch 243
SELECT id, name FROM accounts WHERE id = 1944;
INSERT INTO events (id, kind) VALUES (1945, 'k1945');
UPDATE accounts SET balance = balance + 1946 WHERE id = 1946;
DELETE FROM sessions WHERE expires < 1947;
CREATE TABLE t1948 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1949 TO reader;
SET search_path TO s1950, public;
COMMIT;
-- batch 244
SELECT id, name FROM accounts WHERE id = 1952;
INSERT INTO events (id, kind) VALUES (1953, 'k1953');
UPDATE accounts SET balance = balance + 1954 WHERE id = 1954;
DELETE FROM sessions WHERE expires < 1955;
CREATE TABLE t1956 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1957 TO reader;
SET search_path TO s1958, public;
COMMIT;
-- batch 245
SELECT id, name FROM accounts WHERE id = 1960;
INSERT INTO events (id, kind) VALUES (1961, 'k1961');
UPDATE accounts SET balance = balance + 1962 WHERE id = 1962;
DELETE FROM sessions WHERE expires < 1963;
CREATE TABLE t1964 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1965 TO reader;
SET search_path TO s1966, public;
COMMIT;
-- batch 246
SELECT id, name FROM accounts WHERE id = 1968;
INSERT INTO events (id, kind) VALUES (1969, 'k1969');
UPDATE accounts SET balance = balance + 1970 WHERE id = 1970;
DELETE FROM sessions WHERE expires < 1971;
CREATE TABLE t1972 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1973 TO reader;
SET search_path TO s1974, public;
COMMIT;
-- batch 247
SELECT id, name FROM accounts WHERE id = 1976;
INSERT INTO events (id, kind) VALUES (1977, 'k1977');
UPDATE accounts SET balance = balance + 1978 WHERE id = 1978;
DELETE FROM sessions WHERE expires < 1979;
CREATE TABLE t1980 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1981 TO reader;
SET search_path TO s1982, public;
COMMIT;
-- batch 248
SELECT id, name FROM accounts WHERE id = 1984;
INSERT INTO events (id, kind) VALUES (1985, 'k1985');
UPDATE accounts SET balance = balance + 1986 WHERE id = 1986;
DELETE FROM sessions WHERE expires < 1987;
CREATE TABLE t1988 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1989 TO reader;
SET search_path TO s1990, public;
COMMIT;
-- batch 249
SELECT id, name FROM accounts WHERE id = 1992;
INSERT INTO events (id, kind) VALUES (1993, 'k1993');
UPDATE accounts SET balance = balance + 1994 WHERE id = 1994;
DELETE FROM sessions WHERE expires < 1995;
CREATE TABLE t1996 (id int PRIMARY KEY, v text);
GRANT SELECT ON t1997 TO reader;
SET search_path TO s1998, public;
COMMIT;
