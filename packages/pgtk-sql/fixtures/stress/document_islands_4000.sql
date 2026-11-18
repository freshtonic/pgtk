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
-- batch 250
SELECT id, name FROM accounts WHERE id = 2000;
INSERT INTO events (id, kind) VALUES (2001, 'k2001');
UPDATE accounts SET balance = balance + 2002 WHERE id = 2002;
DELETE FROM sessions WHERE expires < 2003;
CREATE TABLE t2004 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2005 TO reader;
SET search_path TO s2006, public;
COMMIT;
-- batch 251
SELECT id, name FROM accounts WHERE id = 2008;
INSERT INTO events (id, kind) VALUES (2009, 'k2009');
UPDATE accounts SET balance = balance + 2010 WHERE id = 2010;
DELETE FROM sessions WHERE expires < 2011;
CREATE TABLE t2012 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2013 TO reader;
SET search_path TO s2014, public;
COMMIT;
-- batch 252
SELECT id, name FROM accounts WHERE id = 2016;
INSERT INTO events (id, kind) VALUES (2017, 'k2017');
UPDATE accounts SET balance = balance + 2018 WHERE id = 2018;
DELETE FROM sessions WHERE expires < 2019;
CREATE TABLE t2020 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2021 TO reader;
SET search_path TO s2022, public;
COMMIT;
-- batch 253
SELECT id, name FROM accounts WHERE id = 2024;
INSERT INTO events (id, kind) VALUES (2025, 'k2025');
UPDATE accounts SET balance = balance + 2026 WHERE id = 2026;
DELETE FROM sessions WHERE expires < 2027;
CREATE TABLE t2028 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2029 TO reader;
SET search_path TO s2030, public;
COMMIT;
-- batch 254
SELECT id, name FROM accounts WHERE id = 2032;
INSERT INTO events (id, kind) VALUES (2033, 'k2033');
UPDATE accounts SET balance = balance + 2034 WHERE id = 2034;
DELETE FROM sessions WHERE expires < 2035;
CREATE TABLE t2036 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2037 TO reader;
SET search_path TO s2038, public;
COMMIT;
-- batch 255
SELECT id, name FROM accounts WHERE id = 2040;
INSERT INTO events (id, kind) VALUES (2041, 'k2041');
UPDATE accounts SET balance = balance + 2042 WHERE id = 2042;
DELETE FROM sessions WHERE expires < 2043;
CREATE TABLE t2044 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2045 TO reader;
SET search_path TO s2046, public;
COMMIT;
-- batch 256
SELECT id, name FROM accounts WHERE id = 2048;
INSERT INTO events (id, kind) VALUES (2049, 'k2049');
UPDATE accounts SET balance = balance + 2050 WHERE id = 2050;
DELETE FROM sessions WHERE expires < 2051;
CREATE TABLE t2052 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2053 TO reader;
SET search_path TO s2054, public;
COMMIT;
-- batch 257
SELECT id, name FROM accounts WHERE id = 2056;
INSERT INTO events (id, kind) VALUES (2057, 'k2057');
UPDATE accounts SET balance = balance + 2058 WHERE id = 2058;
DELETE FROM sessions WHERE expires < 2059;
CREATE TABLE t2060 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2061 TO reader;
SET search_path TO s2062, public;
COMMIT;
-- batch 258
SELECT id, name FROM accounts WHERE id = 2064;
INSERT INTO events (id, kind) VALUES (2065, 'k2065');
UPDATE accounts SET balance = balance + 2066 WHERE id = 2066;
DELETE FROM sessions WHERE expires < 2067;
CREATE TABLE t2068 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2069 TO reader;
SET search_path TO s2070, public;
COMMIT;
-- batch 259
SELECT id, name FROM accounts WHERE id = 2072;
INSERT INTO events (id, kind) VALUES (2073, 'k2073');
UPDATE accounts SET balance = balance + 2074 WHERE id = 2074;
DELETE FROM sessions WHERE expires < 2075;
CREATE TABLE t2076 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2077 TO reader;
SET search_path TO s2078, public;
COMMIT;
-- batch 260
SELECT id, name FROM accounts WHERE id = 2080;
INSERT INTO events (id, kind) VALUES (2081, 'k2081');
UPDATE accounts SET balance = balance + 2082 WHERE id = 2082;
DELETE FROM sessions WHERE expires < 2083;
CREATE TABLE t2084 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2085 TO reader;
SET search_path TO s2086, public;
COMMIT;
-- batch 261
SELECT id, name FROM accounts WHERE id = 2088;
INSERT INTO events (id, kind) VALUES (2089, 'k2089');
UPDATE accounts SET balance = balance + 2090 WHERE id = 2090;
DELETE FROM sessions WHERE expires < 2091;
CREATE TABLE t2092 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2093 TO reader;
SET search_path TO s2094, public;
COMMIT;
-- batch 262
SELECT id, name FROM accounts WHERE id = 2096;
INSERT INTO events (id, kind) VALUES (2097, 'k2097');
UPDATE accounts SET balance = balance + 2098 WHERE id = 2098;
DELETE FROM sessions WHERE expires < 2099;
CREATE TABLE t2100 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2101 TO reader;
SET search_path TO s2102, public;
COMMIT;
-- batch 263
SELECT id, name FROM accounts WHERE id = 2104;
INSERT INTO events (id, kind) VALUES (2105, 'k2105');
UPDATE accounts SET balance = balance + 2106 WHERE id = 2106;
DELETE FROM sessions WHERE expires < 2107;
CREATE TABLE t2108 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2109 TO reader;
SET search_path TO s2110, public;
COMMIT;
-- batch 264
SELECT id, name FROM accounts WHERE id = 2112;
INSERT INTO events (id, kind) VALUES (2113, 'k2113');
UPDATE accounts SET balance = balance + 2114 WHERE id = 2114;
DELETE FROM sessions WHERE expires < 2115;
CREATE TABLE t2116 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2117 TO reader;
SET search_path TO s2118, public;
COMMIT;
-- batch 265
SELECT id, name FROM accounts WHERE id = 2120;
INSERT INTO events (id, kind) VALUES (2121, 'k2121');
UPDATE accounts SET balance = balance + 2122 WHERE id = 2122;
DELETE FROM sessions WHERE expires < 2123;
CREATE TABLE t2124 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2125 TO reader;
SET search_path TO s2126, public;
COMMIT;
-- batch 266
SELECT id, name FROM accounts WHERE id = 2128;
INSERT INTO events (id, kind) VALUES (2129, 'k2129');
UPDATE accounts SET balance = balance + 2130 WHERE id = 2130;
DELETE FROM sessions WHERE expires < 2131;
CREATE TABLE t2132 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2133 TO reader;
SET search_path TO s2134, public;
COMMIT;
-- batch 267
SELECT id, name FROM accounts WHERE id = 2136;
INSERT INTO events (id, kind) VALUES (2137, 'k2137');
UPDATE accounts SET balance = balance + 2138 WHERE id = 2138;
DELETE FROM sessions WHERE expires < 2139;
CREATE TABLE t2140 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2141 TO reader;
SET search_path TO s2142, public;
COMMIT;
-- batch 268
SELECT id, name FROM accounts WHERE id = 2144;
INSERT INTO events (id, kind) VALUES (2145, 'k2145');
UPDATE accounts SET balance = balance + 2146 WHERE id = 2146;
DELETE FROM sessions WHERE expires < 2147;
CREATE TABLE t2148 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2149 TO reader;
SET search_path TO s2150, public;
COMMIT;
-- batch 269
SELECT id, name FROM accounts WHERE id = 2152;
INSERT INTO events (id, kind) VALUES (2153, 'k2153');
UPDATE accounts SET balance = balance + 2154 WHERE id = 2154;
DELETE FROM sessions WHERE expires < 2155;
CREATE TABLE t2156 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2157 TO reader;
SET search_path TO s2158, public;
COMMIT;
-- batch 270
SELECT id, name FROM accounts WHERE id = 2160;
INSERT INTO events (id, kind) VALUES (2161, 'k2161');
UPDATE accounts SET balance = balance + 2162 WHERE id = 2162;
DELETE FROM sessions WHERE expires < 2163;
CREATE TABLE t2164 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2165 TO reader;
SET search_path TO s2166, public;
COMMIT;
-- batch 271
SELECT id, name FROM accounts WHERE id = 2168;
INSERT INTO events (id, kind) VALUES (2169, 'k2169');
UPDATE accounts SET balance = balance + 2170 WHERE id = 2170;
DELETE FROM sessions WHERE expires < 2171;
CREATE TABLE t2172 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2173 TO reader;
SET search_path TO s2174, public;
COMMIT;
-- batch 272
SELECT id, name FROM accounts WHERE id = 2176;
INSERT INTO events (id, kind) VALUES (2177, 'k2177');
UPDATE accounts SET balance = balance + 2178 WHERE id = 2178;
DELETE FROM sessions WHERE expires < 2179;
CREATE TABLE t2180 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2181 TO reader;
SET search_path TO s2182, public;
COMMIT;
-- batch 273
SELECT id, name FROM accounts WHERE id = 2184;
INSERT INTO events (id, kind) VALUES (2185, 'k2185');
UPDATE accounts SET balance = balance + 2186 WHERE id = 2186;
DELETE FROM sessions WHERE expires < 2187;
CREATE TABLE t2188 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2189 TO reader;
SET search_path TO s2190, public;
COMMIT;
-- batch 274
SELECT id, name FROM accounts WHERE id = 2192;
INSERT INTO events (id, kind) VALUES (2193, 'k2193');
UPDATE accounts SET balance = balance + 2194 WHERE id = 2194;
DELETE FROM sessions WHERE expires < 2195;
CREATE TABLE t2196 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2197 TO reader;
SET search_path TO s2198, public;
COMMIT;
-- batch 275
SELECT id, name FROM accounts WHERE id = 2200;
INSERT INTO events (id, kind) VALUES (2201, 'k2201');
UPDATE accounts SET balance = balance + 2202 WHERE id = 2202;
DELETE FROM sessions WHERE expires < 2203;
CREATE TABLE t2204 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2205 TO reader;
SET search_path TO s2206, public;
COMMIT;
-- batch 276
SELECT id, name FROM accounts WHERE id = 2208;
INSERT INTO events (id, kind) VALUES (2209, 'k2209');
UPDATE accounts SET balance = balance + 2210 WHERE id = 2210;
DELETE FROM sessions WHERE expires < 2211;
CREATE TABLE t2212 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2213 TO reader;
SET search_path TO s2214, public;
COMMIT;
-- batch 277
SELECT id, name FROM accounts WHERE id = 2216;
INSERT INTO events (id, kind) VALUES (2217, 'k2217');
UPDATE accounts SET balance = balance + 2218 WHERE id = 2218;
DELETE FROM sessions WHERE expires < 2219;
CREATE TABLE t2220 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2221 TO reader;
SET search_path TO s2222, public;
COMMIT;
-- batch 278
SELECT id, name FROM accounts WHERE id = 2224;
INSERT INTO events (id, kind) VALUES (2225, 'k2225');
UPDATE accounts SET balance = balance + 2226 WHERE id = 2226;
DELETE FROM sessions WHERE expires < 2227;
CREATE TABLE t2228 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2229 TO reader;
SET search_path TO s2230, public;
COMMIT;
-- batch 279
SELECT id, name FROM accounts WHERE id = 2232;
INSERT INTO events (id, kind) VALUES (2233, 'k2233');
UPDATE accounts SET balance = balance + 2234 WHERE id = 2234;
DELETE FROM sessions WHERE expires < 2235;
CREATE TABLE t2236 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2237 TO reader;
SET search_path TO s2238, public;
COMMIT;
-- batch 280
SELECT id, name FROM accounts WHERE id = 2240;
INSERT INTO events (id, kind) VALUES (2241, 'k2241');
UPDATE accounts SET balance = balance + 2242 WHERE id = 2242;
DELETE FROM sessions WHERE expires < 2243;
CREATE TABLE t2244 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2245 TO reader;
SET search_path TO s2246, public;
COMMIT;
-- batch 281
SELECT id, name FROM accounts WHERE id = 2248;
INSERT INTO events (id, kind) VALUES (2249, 'k2249');
UPDATE accounts SET balance = balance + 2250 WHERE id = 2250;
DELETE FROM sessions WHERE expires < 2251;
CREATE TABLE t2252 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2253 TO reader;
SET search_path TO s2254, public;
COMMIT;
-- batch 282
SELECT id, name FROM accounts WHERE id = 2256;
INSERT INTO events (id, kind) VALUES (2257, 'k2257');
UPDATE accounts SET balance = balance + 2258 WHERE id = 2258;
DELETE FROM sessions WHERE expires < 2259;
CREATE TABLE t2260 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2261 TO reader;
SET search_path TO s2262, public;
COMMIT;
-- batch 283
SELECT id, name FROM accounts WHERE id = 2264;
INSERT INTO events (id, kind) VALUES (2265, 'k2265');
UPDATE accounts SET balance = balance + 2266 WHERE id = 2266;
DELETE FROM sessions WHERE expires < 2267;
CREATE TABLE t2268 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2269 TO reader;
SET search_path TO s2270, public;
COMMIT;
-- batch 284
SELECT id, name FROM accounts WHERE id = 2272;
INSERT INTO events (id, kind) VALUES (2273, 'k2273');
UPDATE accounts SET balance = balance + 2274 WHERE id = 2274;
DELETE FROM sessions WHERE expires < 2275;
CREATE TABLE t2276 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2277 TO reader;
SET search_path TO s2278, public;
COMMIT;
-- batch 285
SELECT id, name FROM accounts WHERE id = 2280;
INSERT INTO events (id, kind) VALUES (2281, 'k2281');
UPDATE accounts SET balance = balance + 2282 WHERE id = 2282;
DELETE FROM sessions WHERE expires < 2283;
CREATE TABLE t2284 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2285 TO reader;
SET search_path TO s2286, public;
COMMIT;
-- batch 286
SELECT id, name FROM accounts WHERE id = 2288;
INSERT INTO events (id, kind) VALUES (2289, 'k2289');
UPDATE accounts SET balance = balance + 2290 WHERE id = 2290;
DELETE FROM sessions WHERE expires < 2291;
CREATE TABLE t2292 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2293 TO reader;
SET search_path TO s2294, public;
COMMIT;
-- batch 287
SELECT id, name FROM accounts WHERE id = 2296;
INSERT INTO events (id, kind) VALUES (2297, 'k2297');
UPDATE accounts SET balance = balance + 2298 WHERE id = 2298;
DELETE FROM sessions WHERE expires < 2299;
CREATE TABLE t2300 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2301 TO reader;
SET search_path TO s2302, public;
COMMIT;
-- batch 288
SELECT id, name FROM accounts WHERE id = 2304;
INSERT INTO events (id, kind) VALUES (2305, 'k2305');
UPDATE accounts SET balance = balance + 2306 WHERE id = 2306;
DELETE FROM sessions WHERE expires < 2307;
CREATE TABLE t2308 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2309 TO reader;
SET search_path TO s2310, public;
COMMIT;
-- batch 289
SELECT id, name FROM accounts WHERE id = 2312;
INSERT INTO events (id, kind) VALUES (2313, 'k2313');
UPDATE accounts SET balance = balance + 2314 WHERE id = 2314;
DELETE FROM sessions WHERE expires < 2315;
CREATE TABLE t2316 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2317 TO reader;
SET search_path TO s2318, public;
COMMIT;
-- batch 290
SELECT id, name FROM accounts WHERE id = 2320;
INSERT INTO events (id, kind) VALUES (2321, 'k2321');
UPDATE accounts SET balance = balance + 2322 WHERE id = 2322;
DELETE FROM sessions WHERE expires < 2323;
CREATE TABLE t2324 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2325 TO reader;
SET search_path TO s2326, public;
COMMIT;
-- batch 291
SELECT id, name FROM accounts WHERE id = 2328;
INSERT INTO events (id, kind) VALUES (2329, 'k2329');
UPDATE accounts SET balance = balance + 2330 WHERE id = 2330;
DELETE FROM sessions WHERE expires < 2331;
CREATE TABLE t2332 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2333 TO reader;
SET search_path TO s2334, public;
COMMIT;
-- batch 292
SELECT id, name FROM accounts WHERE id = 2336;
INSERT INTO events (id, kind) VALUES (2337, 'k2337');
UPDATE accounts SET balance = balance + 2338 WHERE id = 2338;
DELETE FROM sessions WHERE expires < 2339;
CREATE TABLE t2340 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2341 TO reader;
SET search_path TO s2342, public;
COMMIT;
-- batch 293
SELECT id, name FROM accounts WHERE id = 2344;
INSERT INTO events (id, kind) VALUES (2345, 'k2345');
UPDATE accounts SET balance = balance + 2346 WHERE id = 2346;
DELETE FROM sessions WHERE expires < 2347;
CREATE TABLE t2348 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2349 TO reader;
SET search_path TO s2350, public;
COMMIT;
-- batch 294
SELECT id, name FROM accounts WHERE id = 2352;
INSERT INTO events (id, kind) VALUES (2353, 'k2353');
UPDATE accounts SET balance = balance + 2354 WHERE id = 2354;
DELETE FROM sessions WHERE expires < 2355;
CREATE TABLE t2356 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2357 TO reader;
SET search_path TO s2358, public;
COMMIT;
-- batch 295
SELECT id, name FROM accounts WHERE id = 2360;
INSERT INTO events (id, kind) VALUES (2361, 'k2361');
UPDATE accounts SET balance = balance + 2362 WHERE id = 2362;
DELETE FROM sessions WHERE expires < 2363;
CREATE TABLE t2364 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2365 TO reader;
SET search_path TO s2366, public;
COMMIT;
-- batch 296
SELECT id, name FROM accounts WHERE id = 2368;
INSERT INTO events (id, kind) VALUES (2369, 'k2369');
UPDATE accounts SET balance = balance + 2370 WHERE id = 2370;
DELETE FROM sessions WHERE expires < 2371;
CREATE TABLE t2372 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2373 TO reader;
SET search_path TO s2374, public;
COMMIT;
-- batch 297
SELECT id, name FROM accounts WHERE id = 2376;
INSERT INTO events (id, kind) VALUES (2377, 'k2377');
UPDATE accounts SET balance = balance + 2378 WHERE id = 2378;
DELETE FROM sessions WHERE expires < 2379;
CREATE TABLE t2380 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2381 TO reader;
SET search_path TO s2382, public;
COMMIT;
-- batch 298
SELECT id, name FROM accounts WHERE id = 2384;
INSERT INTO events (id, kind) VALUES (2385, 'k2385');
UPDATE accounts SET balance = balance + 2386 WHERE id = 2386;
DELETE FROM sessions WHERE expires < 2387;
CREATE TABLE t2388 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2389 TO reader;
SET search_path TO s2390, public;
COMMIT;
-- batch 299
SELECT id, name FROM accounts WHERE id = 2392;
INSERT INTO events (id, kind) VALUES (2393, 'k2393');
UPDATE accounts SET balance = balance + 2394 WHERE id = 2394;
DELETE FROM sessions WHERE expires < 2395;
CREATE TABLE t2396 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2397 TO reader;
SET search_path TO s2398, public;
COMMIT;
-- batch 300
SELECT id, name FROM accounts WHERE id = 2400;
INSERT INTO events (id, kind) VALUES (2401, 'k2401');
UPDATE accounts SET balance = balance + 2402 WHERE id = 2402;
DELETE FROM sessions WHERE expires < 2403;
CREATE TABLE t2404 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2405 TO reader;
SET search_path TO s2406, public;
COMMIT;
-- batch 301
SELECT id, name FROM accounts WHERE id = 2408;
INSERT INTO events (id, kind) VALUES (2409, 'k2409');
UPDATE accounts SET balance = balance + 2410 WHERE id = 2410;
DELETE FROM sessions WHERE expires < 2411;
CREATE TABLE t2412 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2413 TO reader;
SET search_path TO s2414, public;
COMMIT;
-- batch 302
SELECT id, name FROM accounts WHERE id = 2416;
INSERT INTO events (id, kind) VALUES (2417, 'k2417');
UPDATE accounts SET balance = balance + 2418 WHERE id = 2418;
DELETE FROM sessions WHERE expires < 2419;
CREATE TABLE t2420 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2421 TO reader;
SET search_path TO s2422, public;
COMMIT;
-- batch 303
SELECT id, name FROM accounts WHERE id = 2424;
INSERT INTO events (id, kind) VALUES (2425, 'k2425');
UPDATE accounts SET balance = balance + 2426 WHERE id = 2426;
DELETE FROM sessions WHERE expires < 2427;
CREATE TABLE t2428 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2429 TO reader;
SET search_path TO s2430, public;
COMMIT;
-- batch 304
SELECT id, name FROM accounts WHERE id = 2432;
INSERT INTO events (id, kind) VALUES (2433, 'k2433');
UPDATE accounts SET balance = balance + 2434 WHERE id = 2434;
DELETE FROM sessions WHERE expires < 2435;
CREATE TABLE t2436 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2437 TO reader;
SET search_path TO s2438, public;
COMMIT;
-- batch 305
SELECT id, name FROM accounts WHERE id = 2440;
INSERT INTO events (id, kind) VALUES (2441, 'k2441');
UPDATE accounts SET balance = balance + 2442 WHERE id = 2442;
DELETE FROM sessions WHERE expires < 2443;
CREATE TABLE t2444 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2445 TO reader;
SET search_path TO s2446, public;
COMMIT;
-- batch 306
SELECT id, name FROM accounts WHERE id = 2448;
INSERT INTO events (id, kind) VALUES (2449, 'k2449');
UPDATE accounts SET balance = balance + 2450 WHERE id = 2450;
DELETE FROM sessions WHERE expires < 2451;
CREATE TABLE t2452 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2453 TO reader;
SET search_path TO s2454, public;
COMMIT;
-- batch 307
SELECT id, name FROM accounts WHERE id = 2456;
INSERT INTO events (id, kind) VALUES (2457, 'k2457');
UPDATE accounts SET balance = balance + 2458 WHERE id = 2458;
DELETE FROM sessions WHERE expires < 2459;
CREATE TABLE t2460 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2461 TO reader;
SET search_path TO s2462, public;
COMMIT;
-- batch 308
SELECT id, name FROM accounts WHERE id = 2464;
INSERT INTO events (id, kind) VALUES (2465, 'k2465');
UPDATE accounts SET balance = balance + 2466 WHERE id = 2466;
DELETE FROM sessions WHERE expires < 2467;
CREATE TABLE t2468 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2469 TO reader;
SET search_path TO s2470, public;
COMMIT;
-- batch 309
SELECT id, name FROM accounts WHERE id = 2472;
INSERT INTO events (id, kind) VALUES (2473, 'k2473');
UPDATE accounts SET balance = balance + 2474 WHERE id = 2474;
DELETE FROM sessions WHERE expires < 2475;
CREATE TABLE t2476 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2477 TO reader;
SET search_path TO s2478, public;
COMMIT;
-- batch 310
SELECT id, name FROM accounts WHERE id = 2480;
INSERT INTO events (id, kind) VALUES (2481, 'k2481');
UPDATE accounts SET balance = balance + 2482 WHERE id = 2482;
DELETE FROM sessions WHERE expires < 2483;
CREATE TABLE t2484 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2485 TO reader;
SET search_path TO s2486, public;
COMMIT;
-- batch 311
SELECT id, name FROM accounts WHERE id = 2488;
INSERT INTO events (id, kind) VALUES (2489, 'k2489');
UPDATE accounts SET balance = balance + 2490 WHERE id = 2490;
DELETE FROM sessions WHERE expires < 2491;
CREATE TABLE t2492 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2493 TO reader;
SET search_path TO s2494, public;
COMMIT;
-- batch 312
SELECT id, name FROM accounts WHERE id = 2496;
INSERT INTO events (id, kind) VALUES (2497, 'k2497');
UPDATE accounts SET balance = balance + 2498 WHERE id = 2498;
DELETE FROM sessions WHERE expires < 2499;
CREATE TABLE t2500 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2501 TO reader;
SET search_path TO s2502, public;
COMMIT;
-- batch 313
SELECT id, name FROM accounts WHERE id = 2504;
INSERT INTO events (id, kind) VALUES (2505, 'k2505');
UPDATE accounts SET balance = balance + 2506 WHERE id = 2506;
DELETE FROM sessions WHERE expires < 2507;
CREATE TABLE t2508 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2509 TO reader;
SET search_path TO s2510, public;
COMMIT;
-- batch 314
SELECT id, name FROM accounts WHERE id = 2512;
INSERT INTO events (id, kind) VALUES (2513, 'k2513');
UPDATE accounts SET balance = balance + 2514 WHERE id = 2514;
DELETE FROM sessions WHERE expires < 2515;
CREATE TABLE t2516 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2517 TO reader;
SET search_path TO s2518, public;
COMMIT;
-- batch 315
SELECT id, name FROM accounts WHERE id = 2520;
INSERT INTO events (id, kind) VALUES (2521, 'k2521');
UPDATE accounts SET balance = balance + 2522 WHERE id = 2522;
DELETE FROM sessions WHERE expires < 2523;
CREATE TABLE t2524 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2525 TO reader;
SET search_path TO s2526, public;
COMMIT;
-- batch 316
SELECT id, name FROM accounts WHERE id = 2528;
INSERT INTO events (id, kind) VALUES (2529, 'k2529');
UPDATE accounts SET balance = balance + 2530 WHERE id = 2530;
DELETE FROM sessions WHERE expires < 2531;
CREATE TABLE t2532 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2533 TO reader;
SET search_path TO s2534, public;
COMMIT;
-- batch 317
SELECT id, name FROM accounts WHERE id = 2536;
INSERT INTO events (id, kind) VALUES (2537, 'k2537');
UPDATE accounts SET balance = balance + 2538 WHERE id = 2538;
DELETE FROM sessions WHERE expires < 2539;
CREATE TABLE t2540 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2541 TO reader;
SET search_path TO s2542, public;
COMMIT;
-- batch 318
SELECT id, name FROM accounts WHERE id = 2544;
INSERT INTO events (id, kind) VALUES (2545, 'k2545');
UPDATE accounts SET balance = balance + 2546 WHERE id = 2546;
DELETE FROM sessions WHERE expires < 2547;
CREATE TABLE t2548 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2549 TO reader;
SET search_path TO s2550, public;
COMMIT;
-- batch 319
SELECT id, name FROM accounts WHERE id = 2552;
INSERT INTO events (id, kind) VALUES (2553, 'k2553');
UPDATE accounts SET balance = balance + 2554 WHERE id = 2554;
DELETE FROM sessions WHERE expires < 2555;
CREATE TABLE t2556 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2557 TO reader;
SET search_path TO s2558, public;
COMMIT;
-- batch 320
SELECT id, name FROM accounts WHERE id = 2560;
INSERT INTO events (id, kind) VALUES (2561, 'k2561');
UPDATE accounts SET balance = balance + 2562 WHERE id = 2562;
DELETE FROM sessions WHERE expires < 2563;
CREATE TABLE t2564 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2565 TO reader;
SET search_path TO s2566, public;
COMMIT;
-- batch 321
SELECT id, name FROM accounts WHERE id = 2568;
INSERT INTO events (id, kind) VALUES (2569, 'k2569');
UPDATE accounts SET balance = balance + 2570 WHERE id = 2570;
DELETE FROM sessions WHERE expires < 2571;
CREATE TABLE t2572 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2573 TO reader;
SET search_path TO s2574, public;
COMMIT;
-- batch 322
SELECT id, name FROM accounts WHERE id = 2576;
INSERT INTO events (id, kind) VALUES (2577, 'k2577');
UPDATE accounts SET balance = balance + 2578 WHERE id = 2578;
DELETE FROM sessions WHERE expires < 2579;
CREATE TABLE t2580 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2581 TO reader;
SET search_path TO s2582, public;
COMMIT;
-- batch 323
SELECT id, name FROM accounts WHERE id = 2584;
INSERT INTO events (id, kind) VALUES (2585, 'k2585');
UPDATE accounts SET balance = balance + 2586 WHERE id = 2586;
DELETE FROM sessions WHERE expires < 2587;
CREATE TABLE t2588 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2589 TO reader;
SET search_path TO s2590, public;
COMMIT;
-- batch 324
SELECT id, name FROM accounts WHERE id = 2592;
INSERT INTO events (id, kind) VALUES (2593, 'k2593');
UPDATE accounts SET balance = balance + 2594 WHERE id = 2594;
DELETE FROM sessions WHERE expires < 2595;
CREATE TABLE t2596 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2597 TO reader;
SET search_path TO s2598, public;
COMMIT;
-- batch 325
SELECT id, name FROM accounts WHERE id = 2600;
INSERT INTO events (id, kind) VALUES (2601, 'k2601');
UPDATE accounts SET balance = balance + 2602 WHERE id = 2602;
DELETE FROM sessions WHERE expires < 2603;
CREATE TABLE t2604 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2605 TO reader;
SET search_path TO s2606, public;
COMMIT;
-- batch 326
SELECT id, name FROM accounts WHERE id = 2608;
INSERT INTO events (id, kind) VALUES (2609, 'k2609');
UPDATE accounts SET balance = balance + 2610 WHERE id = 2610;
DELETE FROM sessions WHERE expires < 2611;
CREATE TABLE t2612 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2613 TO reader;
SET search_path TO s2614, public;
COMMIT;
-- batch 327
SELECT id, name FROM accounts WHERE id = 2616;
INSERT INTO events (id, kind) VALUES (2617, 'k2617');
UPDATE accounts SET balance = balance + 2618 WHERE id = 2618;
DELETE FROM sessions WHERE expires < 2619;
CREATE TABLE t2620 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2621 TO reader;
SET search_path TO s2622, public;
COMMIT;
-- batch 328
SELECT id, name FROM accounts WHERE id = 2624;
INSERT INTO events (id, kind) VALUES (2625, 'k2625');
UPDATE accounts SET balance = balance + 2626 WHERE id = 2626;
DELETE FROM sessions WHERE expires < 2627;
CREATE TABLE t2628 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2629 TO reader;
SET search_path TO s2630, public;
COMMIT;
-- batch 329
SELECT id, name FROM accounts WHERE id = 2632;
INSERT INTO events (id, kind) VALUES (2633, 'k2633');
UPDATE accounts SET balance = balance + 2634 WHERE id = 2634;
DELETE FROM sessions WHERE expires < 2635;
CREATE TABLE t2636 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2637 TO reader;
SET search_path TO s2638, public;
COMMIT;
-- batch 330
SELECT id, name FROM accounts WHERE id = 2640;
INSERT INTO events (id, kind) VALUES (2641, 'k2641');
UPDATE accounts SET balance = balance + 2642 WHERE id = 2642;
DELETE FROM sessions WHERE expires < 2643;
CREATE TABLE t2644 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2645 TO reader;
SET search_path TO s2646, public;
COMMIT;
-- batch 331
SELECT id, name FROM accounts WHERE id = 2648;
INSERT INTO events (id, kind) VALUES (2649, 'k2649');
UPDATE accounts SET balance = balance + 2650 WHERE id = 2650;
DELETE FROM sessions WHERE expires < 2651;
CREATE TABLE t2652 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2653 TO reader;
SET search_path TO s2654, public;
COMMIT;
-- batch 332
SELECT id, name FROM accounts WHERE id = 2656;
INSERT INTO events (id, kind) VALUES (2657, 'k2657');
UPDATE accounts SET balance = balance + 2658 WHERE id = 2658;
DELETE FROM sessions WHERE expires < 2659;
CREATE TABLE t2660 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2661 TO reader;
SET search_path TO s2662, public;
COMMIT;
-- batch 333
SELECT id, name FROM accounts WHERE id = 2664;
INSERT INTO events (id, kind) VALUES (2665, 'k2665');
UPDATE accounts SET balance = balance + 2666 WHERE id = 2666;
DELETE FROM sessions WHERE expires < 2667;
CREATE TABLE t2668 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2669 TO reader;
SET search_path TO s2670, public;
COMMIT;
-- batch 334
SELECT id, name FROM accounts WHERE id = 2672;
INSERT INTO events (id, kind) VALUES (2673, 'k2673');
UPDATE accounts SET balance = balance + 2674 WHERE id = 2674;
DELETE FROM sessions WHERE expires < 2675;
CREATE TABLE t2676 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2677 TO reader;
SET search_path TO s2678, public;
COMMIT;
-- batch 335
SELECT id, name FROM accounts WHERE id = 2680;
INSERT INTO events (id, kind) VALUES (2681, 'k2681');
UPDATE accounts SET balance = balance + 2682 WHERE id = 2682;
DELETE FROM sessions WHERE expires < 2683;
CREATE TABLE t2684 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2685 TO reader;
SET search_path TO s2686, public;
COMMIT;
-- batch 336
SELECT id, name FROM accounts WHERE id = 2688;
INSERT INTO events (id, kind) VALUES (2689, 'k2689');
UPDATE accounts SET balance = balance + 2690 WHERE id = 2690;
DELETE FROM sessions WHERE expires < 2691;
CREATE TABLE t2692 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2693 TO reader;
SET search_path TO s2694, public;
COMMIT;
-- batch 337
SELECT id, name FROM accounts WHERE id = 2696;
INSERT INTO events (id, kind) VALUES (2697, 'k2697');
UPDATE accounts SET balance = balance + 2698 WHERE id = 2698;
DELETE FROM sessions WHERE expires < 2699;
CREATE TABLE t2700 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2701 TO reader;
SET search_path TO s2702, public;
COMMIT;
-- batch 338
SELECT id, name FROM accounts WHERE id = 2704;
INSERT INTO events (id, kind) VALUES (2705, 'k2705');
UPDATE accounts SET balance = balance + 2706 WHERE id = 2706;
DELETE FROM sessions WHERE expires < 2707;
CREATE TABLE t2708 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2709 TO reader;
SET search_path TO s2710, public;
COMMIT;
-- batch 339
SELECT id, name FROM accounts WHERE id = 2712;
INSERT INTO events (id, kind) VALUES (2713, 'k2713');
UPDATE accounts SET balance = balance + 2714 WHERE id = 2714;
DELETE FROM sessions WHERE expires < 2715;
CREATE TABLE t2716 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2717 TO reader;
SET search_path TO s2718, public;
COMMIT;
-- batch 340
SELECT id, name FROM accounts WHERE id = 2720;
INSERT INTO events (id, kind) VALUES (2721, 'k2721');
UPDATE accounts SET balance = balance + 2722 WHERE id = 2722;
DELETE FROM sessions WHERE expires < 2723;
CREATE TABLE t2724 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2725 TO reader;
SET search_path TO s2726, public;
COMMIT;
-- batch 341
SELECT id, name FROM accounts WHERE id = 2728;
INSERT INTO events (id, kind) VALUES (2729, 'k2729');
UPDATE accounts SET balance = balance + 2730 WHERE id = 2730;
DELETE FROM sessions WHERE expires < 2731;
CREATE TABLE t2732 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2733 TO reader;
SET search_path TO s2734, public;
COMMIT;
-- batch 342
SELECT id, name FROM accounts WHERE id = 2736;
INSERT INTO events (id, kind) VALUES (2737, 'k2737');
UPDATE accounts SET balance = balance + 2738 WHERE id = 2738;
DELETE FROM sessions WHERE expires < 2739;
CREATE TABLE t2740 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2741 TO reader;
SET search_path TO s2742, public;
COMMIT;
-- batch 343
SELECT id, name FROM accounts WHERE id = 2744;
INSERT INTO events (id, kind) VALUES (2745, 'k2745');
UPDATE accounts SET balance = balance + 2746 WHERE id = 2746;
DELETE FROM sessions WHERE expires < 2747;
CREATE TABLE t2748 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2749 TO reader;
SET search_path TO s2750, public;
COMMIT;
-- batch 344
SELECT id, name FROM accounts WHERE id = 2752;
INSERT INTO events (id, kind) VALUES (2753, 'k2753');
UPDATE accounts SET balance = balance + 2754 WHERE id = 2754;
DELETE FROM sessions WHERE expires < 2755;
CREATE TABLE t2756 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2757 TO reader;
SET search_path TO s2758, public;
COMMIT;
-- batch 345
SELECT id, name FROM accounts WHERE id = 2760;
INSERT INTO events (id, kind) VALUES (2761, 'k2761');
UPDATE accounts SET balance = balance + 2762 WHERE id = 2762;
DELETE FROM sessions WHERE expires < 2763;
CREATE TABLE t2764 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2765 TO reader;
SET search_path TO s2766, public;
COMMIT;
-- batch 346
SELECT id, name FROM accounts WHERE id = 2768;
INSERT INTO events (id, kind) VALUES (2769, 'k2769');
UPDATE accounts SET balance = balance + 2770 WHERE id = 2770;
DELETE FROM sessions WHERE expires < 2771;
CREATE TABLE t2772 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2773 TO reader;
SET search_path TO s2774, public;
COMMIT;
-- batch 347
SELECT id, name FROM accounts WHERE id = 2776;
INSERT INTO events (id, kind) VALUES (2777, 'k2777');
UPDATE accounts SET balance = balance + 2778 WHERE id = 2778;
DELETE FROM sessions WHERE expires < 2779;
CREATE TABLE t2780 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2781 TO reader;
SET search_path TO s2782, public;
COMMIT;
-- batch 348
SELECT id, name FROM accounts WHERE id = 2784;
INSERT INTO events (id, kind) VALUES (2785, 'k2785');
UPDATE accounts SET balance = balance + 2786 WHERE id = 2786;
DELETE FROM sessions WHERE expires < 2787;
CREATE TABLE t2788 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2789 TO reader;
SET search_path TO s2790, public;
COMMIT;
-- batch 349
SELECT id, name FROM accounts WHERE id = 2792;
INSERT INTO events (id, kind) VALUES (2793, 'k2793');
UPDATE accounts SET balance = balance + 2794 WHERE id = 2794;
DELETE FROM sessions WHERE expires < 2795;
CREATE TABLE t2796 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2797 TO reader;
SET search_path TO s2798, public;
COMMIT;
-- batch 350
SELECT id, name FROM accounts WHERE id = 2800;
INSERT INTO events (id, kind) VALUES (2801, 'k2801');
UPDATE accounts SET balance = balance + 2802 WHERE id = 2802;
DELETE FROM sessions WHERE expires < 2803;
CREATE TABLE t2804 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2805 TO reader;
SET search_path TO s2806, public;
COMMIT;
-- batch 351
SELECT id, name FROM accounts WHERE id = 2808;
INSERT INTO events (id, kind) VALUES (2809, 'k2809');
UPDATE accounts SET balance = balance + 2810 WHERE id = 2810;
DELETE FROM sessions WHERE expires < 2811;
CREATE TABLE t2812 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2813 TO reader;
SET search_path TO s2814, public;
COMMIT;
-- batch 352
SELECT id, name FROM accounts WHERE id = 2816;
INSERT INTO events (id, kind) VALUES (2817, 'k2817');
UPDATE accounts SET balance = balance + 2818 WHERE id = 2818;
DELETE FROM sessions WHERE expires < 2819;
CREATE TABLE t2820 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2821 TO reader;
SET search_path TO s2822, public;
COMMIT;
-- batch 353
SELECT id, name FROM accounts WHERE id = 2824;
INSERT INTO events (id, kind) VALUES (2825, 'k2825');
UPDATE accounts SET balance = balance + 2826 WHERE id = 2826;
DELETE FROM sessions WHERE expires < 2827;
CREATE TABLE t2828 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2829 TO reader;
SET search_path TO s2830, public;
COMMIT;
-- batch 354
SELECT id, name FROM accounts WHERE id = 2832;
INSERT INTO events (id, kind) VALUES (2833, 'k2833');
UPDATE accounts SET balance = balance + 2834 WHERE id = 2834;
DELETE FROM sessions WHERE expires < 2835;
CREATE TABLE t2836 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2837 TO reader;
SET search_path TO s2838, public;
COMMIT;
-- batch 355
SELECT id, name FROM accounts WHERE id = 2840;
INSERT INTO events (id, kind) VALUES (2841, 'k2841');
UPDATE accounts SET balance = balance + 2842 WHERE id = 2842;
DELETE FROM sessions WHERE expires < 2843;
CREATE TABLE t2844 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2845 TO reader;
SET search_path TO s2846, public;
COMMIT;
-- batch 356
SELECT id, name FROM accounts WHERE id = 2848;
INSERT INTO events (id, kind) VALUES (2849, 'k2849');
UPDATE accounts SET balance = balance + 2850 WHERE id = 2850;
DELETE FROM sessions WHERE expires < 2851;
CREATE TABLE t2852 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2853 TO reader;
SET search_path TO s2854, public;
COMMIT;
-- batch 357
SELECT id, name FROM accounts WHERE id = 2856;
INSERT INTO events (id, kind) VALUES (2857, 'k2857');
UPDATE accounts SET balance = balance + 2858 WHERE id = 2858;
DELETE FROM sessions WHERE expires < 2859;
CREATE TABLE t2860 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2861 TO reader;
SET search_path TO s2862, public;
COMMIT;
-- batch 358
SELECT id, name FROM accounts WHERE id = 2864;
INSERT INTO events (id, kind) VALUES (2865, 'k2865');
UPDATE accounts SET balance = balance + 2866 WHERE id = 2866;
DELETE FROM sessions WHERE expires < 2867;
CREATE TABLE t2868 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2869 TO reader;
SET search_path TO s2870, public;
COMMIT;
-- batch 359
SELECT id, name FROM accounts WHERE id = 2872;
INSERT INTO events (id, kind) VALUES (2873, 'k2873');
UPDATE accounts SET balance = balance + 2874 WHERE id = 2874;
DELETE FROM sessions WHERE expires < 2875;
CREATE TABLE t2876 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2877 TO reader;
SET search_path TO s2878, public;
COMMIT;
-- batch 360
SELECT id, name FROM accounts WHERE id = 2880;
INSERT INTO events (id, kind) VALUES (2881, 'k2881');
UPDATE accounts SET balance = balance + 2882 WHERE id = 2882;
DELETE FROM sessions WHERE expires < 2883;
CREATE TABLE t2884 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2885 TO reader;
SET search_path TO s2886, public;
COMMIT;
-- batch 361
SELECT id, name FROM accounts WHERE id = 2888;
INSERT INTO events (id, kind) VALUES (2889, 'k2889');
UPDATE accounts SET balance = balance + 2890 WHERE id = 2890;
DELETE FROM sessions WHERE expires < 2891;
CREATE TABLE t2892 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2893 TO reader;
SET search_path TO s2894, public;
COMMIT;
-- batch 362
SELECT id, name FROM accounts WHERE id = 2896;
INSERT INTO events (id, kind) VALUES (2897, 'k2897');
UPDATE accounts SET balance = balance + 2898 WHERE id = 2898;
DELETE FROM sessions WHERE expires < 2899;
CREATE TABLE t2900 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2901 TO reader;
SET search_path TO s2902, public;
COMMIT;
-- batch 363
SELECT id, name FROM accounts WHERE id = 2904;
INSERT INTO events (id, kind) VALUES (2905, 'k2905');
UPDATE accounts SET balance = balance + 2906 WHERE id = 2906;
DELETE FROM sessions WHERE expires < 2907;
CREATE TABLE t2908 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2909 TO reader;
SET search_path TO s2910, public;
COMMIT;
-- batch 364
SELECT id, name FROM accounts WHERE id = 2912;
INSERT INTO events (id, kind) VALUES (2913, 'k2913');
UPDATE accounts SET balance = balance + 2914 WHERE id = 2914;
DELETE FROM sessions WHERE expires < 2915;
CREATE TABLE t2916 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2917 TO reader;
SET search_path TO s2918, public;
COMMIT;
-- batch 365
SELECT id, name FROM accounts WHERE id = 2920;
INSERT INTO events (id, kind) VALUES (2921, 'k2921');
UPDATE accounts SET balance = balance + 2922 WHERE id = 2922;
DELETE FROM sessions WHERE expires < 2923;
CREATE TABLE t2924 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2925 TO reader;
SET search_path TO s2926, public;
COMMIT;
-- batch 366
SELECT id, name FROM accounts WHERE id = 2928;
INSERT INTO events (id, kind) VALUES (2929, 'k2929');
UPDATE accounts SET balance = balance + 2930 WHERE id = 2930;
DELETE FROM sessions WHERE expires < 2931;
CREATE TABLE t2932 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2933 TO reader;
SET search_path TO s2934, public;
COMMIT;
-- batch 367
SELECT id, name FROM accounts WHERE id = 2936;
INSERT INTO events (id, kind) VALUES (2937, 'k2937');
UPDATE accounts SET balance = balance + 2938 WHERE id = 2938;
DELETE FROM sessions WHERE expires < 2939;
CREATE TABLE t2940 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2941 TO reader;
SET search_path TO s2942, public;
COMMIT;
-- batch 368
SELECT id, name FROM accounts WHERE id = 2944;
INSERT INTO events (id, kind) VALUES (2945, 'k2945');
UPDATE accounts SET balance = balance + 2946 WHERE id = 2946;
DELETE FROM sessions WHERE expires < 2947;
CREATE TABLE t2948 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2949 TO reader;
SET search_path TO s2950, public;
COMMIT;
-- batch 369
SELECT id, name FROM accounts WHERE id = 2952;
INSERT INTO events (id, kind) VALUES (2953, 'k2953');
UPDATE accounts SET balance = balance + 2954 WHERE id = 2954;
DELETE FROM sessions WHERE expires < 2955;
CREATE TABLE t2956 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2957 TO reader;
SET search_path TO s2958, public;
COMMIT;
-- batch 370
SELECT id, name FROM accounts WHERE id = 2960;
INSERT INTO events (id, kind) VALUES (2961, 'k2961');
UPDATE accounts SET balance = balance + 2962 WHERE id = 2962;
DELETE FROM sessions WHERE expires < 2963;
CREATE TABLE t2964 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2965 TO reader;
SET search_path TO s2966, public;
COMMIT;
-- batch 371
SELECT id, name FROM accounts WHERE id = 2968;
INSERT INTO events (id, kind) VALUES (2969, 'k2969');
UPDATE accounts SET balance = balance + 2970 WHERE id = 2970;
DELETE FROM sessions WHERE expires < 2971;
CREATE TABLE t2972 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2973 TO reader;
SET search_path TO s2974, public;
COMMIT;
-- batch 372
SELECT id, name FROM accounts WHERE id = 2976;
INSERT INTO events (id, kind) VALUES (2977, 'k2977');
UPDATE accounts SET balance = balance + 2978 WHERE id = 2978;
DELETE FROM sessions WHERE expires < 2979;
CREATE TABLE t2980 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2981 TO reader;
SET search_path TO s2982, public;
COMMIT;
-- batch 373
SELECT id, name FROM accounts WHERE id = 2984;
INSERT INTO events (id, kind) VALUES (2985, 'k2985');
UPDATE accounts SET balance = balance + 2986 WHERE id = 2986;
DELETE FROM sessions WHERE expires < 2987;
CREATE TABLE t2988 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2989 TO reader;
SET search_path TO s2990, public;
COMMIT;
-- batch 374
SELECT id, name FROM accounts WHERE id = 2992;
INSERT INTO events (id, kind) VALUES (2993, 'k2993');
UPDATE accounts SET balance = balance + 2994 WHERE id = 2994;
DELETE FROM sessions WHERE expires < 2995;
CREATE TABLE t2996 (id int PRIMARY KEY, v text);
GRANT SELECT ON t2997 TO reader;
SET search_path TO s2998, public;
COMMIT;
-- batch 375
SELECT id, name FROM accounts WHERE id = 3000;
INSERT INTO events (id, kind) VALUES (3001, 'k3001');
UPDATE accounts SET balance = balance + 3002 WHERE id = 3002;
DELETE FROM sessions WHERE expires < 3003;
CREATE TABLE t3004 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3005 TO reader;
SET search_path TO s3006, public;
COMMIT;
-- batch 376
SELECT id, name FROM accounts WHERE id = 3008;
INSERT INTO events (id, kind) VALUES (3009, 'k3009');
UPDATE accounts SET balance = balance + 3010 WHERE id = 3010;
DELETE FROM sessions WHERE expires < 3011;
CREATE TABLE t3012 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3013 TO reader;
SET search_path TO s3014, public;
COMMIT;
-- batch 377
SELECT id, name FROM accounts WHERE id = 3016;
INSERT INTO events (id, kind) VALUES (3017, 'k3017');
UPDATE accounts SET balance = balance + 3018 WHERE id = 3018;
DELETE FROM sessions WHERE expires < 3019;
CREATE TABLE t3020 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3021 TO reader;
SET search_path TO s3022, public;
COMMIT;
-- batch 378
SELECT id, name FROM accounts WHERE id = 3024;
INSERT INTO events (id, kind) VALUES (3025, 'k3025');
UPDATE accounts SET balance = balance + 3026 WHERE id = 3026;
DELETE FROM sessions WHERE expires < 3027;
CREATE TABLE t3028 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3029 TO reader;
SET search_path TO s3030, public;
COMMIT;
-- batch 379
SELECT id, name FROM accounts WHERE id = 3032;
INSERT INTO events (id, kind) VALUES (3033, 'k3033');
UPDATE accounts SET balance = balance + 3034 WHERE id = 3034;
DELETE FROM sessions WHERE expires < 3035;
CREATE TABLE t3036 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3037 TO reader;
SET search_path TO s3038, public;
COMMIT;
-- batch 380
SELECT id, name FROM accounts WHERE id = 3040;
INSERT INTO events (id, kind) VALUES (3041, 'k3041');
UPDATE accounts SET balance = balance + 3042 WHERE id = 3042;
DELETE FROM sessions WHERE expires < 3043;
CREATE TABLE t3044 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3045 TO reader;
SET search_path TO s3046, public;
COMMIT;
-- batch 381
SELECT id, name FROM accounts WHERE id = 3048;
INSERT INTO events (id, kind) VALUES (3049, 'k3049');
UPDATE accounts SET balance = balance + 3050 WHERE id = 3050;
DELETE FROM sessions WHERE expires < 3051;
CREATE TABLE t3052 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3053 TO reader;
SET search_path TO s3054, public;
COMMIT;
-- batch 382
SELECT id, name FROM accounts WHERE id = 3056;
INSERT INTO events (id, kind) VALUES (3057, 'k3057');
UPDATE accounts SET balance = balance + 3058 WHERE id = 3058;
DELETE FROM sessions WHERE expires < 3059;
CREATE TABLE t3060 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3061 TO reader;
SET search_path TO s3062, public;
COMMIT;
-- batch 383
SELECT id, name FROM accounts WHERE id = 3064;
INSERT INTO events (id, kind) VALUES (3065, 'k3065');
UPDATE accounts SET balance = balance + 3066 WHERE id = 3066;
DELETE FROM sessions WHERE expires < 3067;
CREATE TABLE t3068 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3069 TO reader;
SET search_path TO s3070, public;
COMMIT;
-- batch 384
SELECT id, name FROM accounts WHERE id = 3072;
INSERT INTO events (id, kind) VALUES (3073, 'k3073');
UPDATE accounts SET balance = balance + 3074 WHERE id = 3074;
DELETE FROM sessions WHERE expires < 3075;
CREATE TABLE t3076 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3077 TO reader;
SET search_path TO s3078, public;
COMMIT;
-- batch 385
SELECT id, name FROM accounts WHERE id = 3080;
INSERT INTO events (id, kind) VALUES (3081, 'k3081');
UPDATE accounts SET balance = balance + 3082 WHERE id = 3082;
DELETE FROM sessions WHERE expires < 3083;
CREATE TABLE t3084 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3085 TO reader;
SET search_path TO s3086, public;
COMMIT;
-- batch 386
SELECT id, name FROM accounts WHERE id = 3088;
INSERT INTO events (id, kind) VALUES (3089, 'k3089');
UPDATE accounts SET balance = balance + 3090 WHERE id = 3090;
DELETE FROM sessions WHERE expires < 3091;
CREATE TABLE t3092 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3093 TO reader;
SET search_path TO s3094, public;
COMMIT;
-- batch 387
SELECT id, name FROM accounts WHERE id = 3096;
INSERT INTO events (id, kind) VALUES (3097, 'k3097');
UPDATE accounts SET balance = balance + 3098 WHERE id = 3098;
DELETE FROM sessions WHERE expires < 3099;
CREATE TABLE t3100 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3101 TO reader;
SET search_path TO s3102, public;
COMMIT;
-- batch 388
SELECT id, name FROM accounts WHERE id = 3104;
INSERT INTO events (id, kind) VALUES (3105, 'k3105');
UPDATE accounts SET balance = balance + 3106 WHERE id = 3106;
DELETE FROM sessions WHERE expires < 3107;
CREATE TABLE t3108 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3109 TO reader;
SET search_path TO s3110, public;
COMMIT;
-- batch 389
SELECT id, name FROM accounts WHERE id = 3112;
INSERT INTO events (id, kind) VALUES (3113, 'k3113');
UPDATE accounts SET balance = balance + 3114 WHERE id = 3114;
DELETE FROM sessions WHERE expires < 3115;
CREATE TABLE t3116 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3117 TO reader;
SET search_path TO s3118, public;
COMMIT;
-- batch 390
SELECT id, name FROM accounts WHERE id = 3120;
INSERT INTO events (id, kind) VALUES (3121, 'k3121');
UPDATE accounts SET balance = balance + 3122 WHERE id = 3122;
DELETE FROM sessions WHERE expires < 3123;
CREATE TABLE t3124 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3125 TO reader;
SET search_path TO s3126, public;
COMMIT;
-- batch 391
SELECT id, name FROM accounts WHERE id = 3128;
INSERT INTO events (id, kind) VALUES (3129, 'k3129');
UPDATE accounts SET balance = balance + 3130 WHERE id = 3130;
DELETE FROM sessions WHERE expires < 3131;
CREATE TABLE t3132 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3133 TO reader;
SET search_path TO s3134, public;
COMMIT;
-- batch 392
SELECT id, name FROM accounts WHERE id = 3136;
INSERT INTO events (id, kind) VALUES (3137, 'k3137');
UPDATE accounts SET balance = balance + 3138 WHERE id = 3138;
DELETE FROM sessions WHERE expires < 3139;
CREATE TABLE t3140 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3141 TO reader;
SET search_path TO s3142, public;
COMMIT;
-- batch 393
SELECT id, name FROM accounts WHERE id = 3144;
INSERT INTO events (id, kind) VALUES (3145, 'k3145');
UPDATE accounts SET balance = balance + 3146 WHERE id = 3146;
DELETE FROM sessions WHERE expires < 3147;
CREATE TABLE t3148 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3149 TO reader;
SET search_path TO s3150, public;
COMMIT;
-- batch 394
SELECT id, name FROM accounts WHERE id = 3152;
INSERT INTO events (id, kind) VALUES (3153, 'k3153');
UPDATE accounts SET balance = balance + 3154 WHERE id = 3154;
DELETE FROM sessions WHERE expires < 3155;
CREATE TABLE t3156 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3157 TO reader;
SET search_path TO s3158, public;
COMMIT;
-- batch 395
SELECT id, name FROM accounts WHERE id = 3160;
INSERT INTO events (id, kind) VALUES (3161, 'k3161');
UPDATE accounts SET balance = balance + 3162 WHERE id = 3162;
DELETE FROM sessions WHERE expires < 3163;
CREATE TABLE t3164 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3165 TO reader;
SET search_path TO s3166, public;
COMMIT;
-- batch 396
SELECT id, name FROM accounts WHERE id = 3168;
INSERT INTO events (id, kind) VALUES (3169, 'k3169');
UPDATE accounts SET balance = balance + 3170 WHERE id = 3170;
DELETE FROM sessions WHERE expires < 3171;
CREATE TABLE t3172 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3173 TO reader;
SET search_path TO s3174, public;
COMMIT;
-- batch 397
SELECT id, name FROM accounts WHERE id = 3176;
INSERT INTO events (id, kind) VALUES (3177, 'k3177');
UPDATE accounts SET balance = balance + 3178 WHERE id = 3178;
DELETE FROM sessions WHERE expires < 3179;
CREATE TABLE t3180 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3181 TO reader;
SET search_path TO s3182, public;
COMMIT;
-- batch 398
SELECT id, name FROM accounts WHERE id = 3184;
INSERT INTO events (id, kind) VALUES (3185, 'k3185');
UPDATE accounts SET balance = balance + 3186 WHERE id = 3186;
DELETE FROM sessions WHERE expires < 3187;
CREATE TABLE t3188 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3189 TO reader;
SET search_path TO s3190, public;
COMMIT;
-- batch 399
SELECT id, name FROM accounts WHERE id = 3192;
INSERT INTO events (id, kind) VALUES (3193, 'k3193');
UPDATE accounts SET balance = balance + 3194 WHERE id = 3194;
DELETE FROM sessions WHERE expires < 3195;
CREATE TABLE t3196 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3197 TO reader;
SET search_path TO s3198, public;
COMMIT;
-- batch 400
SELECT id, name FROM accounts WHERE id = 3200;
INSERT INTO events (id, kind) VALUES (3201, 'k3201');
UPDATE accounts SET balance = balance + 3202 WHERE id = 3202;
DELETE FROM sessions WHERE expires < 3203;
CREATE TABLE t3204 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3205 TO reader;
SET search_path TO s3206, public;
COMMIT;
-- batch 401
SELECT id, name FROM accounts WHERE id = 3208;
INSERT INTO events (id, kind) VALUES (3209, 'k3209');
UPDATE accounts SET balance = balance + 3210 WHERE id = 3210;
DELETE FROM sessions WHERE expires < 3211;
CREATE TABLE t3212 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3213 TO reader;
SET search_path TO s3214, public;
COMMIT;
-- batch 402
SELECT id, name FROM accounts WHERE id = 3216;
INSERT INTO events (id, kind) VALUES (3217, 'k3217');
UPDATE accounts SET balance = balance + 3218 WHERE id = 3218;
DELETE FROM sessions WHERE expires < 3219;
CREATE TABLE t3220 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3221 TO reader;
SET search_path TO s3222, public;
COMMIT;
-- batch 403
SELECT id, name FROM accounts WHERE id = 3224;
INSERT INTO events (id, kind) VALUES (3225, 'k3225');
UPDATE accounts SET balance = balance + 3226 WHERE id = 3226;
DELETE FROM sessions WHERE expires < 3227;
CREATE TABLE t3228 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3229 TO reader;
SET search_path TO s3230, public;
COMMIT;
-- batch 404
SELECT id, name FROM accounts WHERE id = 3232;
INSERT INTO events (id, kind) VALUES (3233, 'k3233');
UPDATE accounts SET balance = balance + 3234 WHERE id = 3234;
DELETE FROM sessions WHERE expires < 3235;
CREATE TABLE t3236 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3237 TO reader;
SET search_path TO s3238, public;
COMMIT;
-- batch 405
SELECT id, name FROM accounts WHERE id = 3240;
INSERT INTO events (id, kind) VALUES (3241, 'k3241');
UPDATE accounts SET balance = balance + 3242 WHERE id = 3242;
DELETE FROM sessions WHERE expires < 3243;
CREATE TABLE t3244 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3245 TO reader;
SET search_path TO s3246, public;
COMMIT;
-- batch 406
SELECT id, name FROM accounts WHERE id = 3248;
INSERT INTO events (id, kind) VALUES (3249, 'k3249');
UPDATE accounts SET balance = balance + 3250 WHERE id = 3250;
DELETE FROM sessions WHERE expires < 3251;
CREATE TABLE t3252 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3253 TO reader;
SET search_path TO s3254, public;
COMMIT;
-- batch 407
SELECT id, name FROM accounts WHERE id = 3256;
INSERT INTO events (id, kind) VALUES (3257, 'k3257');
UPDATE accounts SET balance = balance + 3258 WHERE id = 3258;
DELETE FROM sessions WHERE expires < 3259;
CREATE TABLE t3260 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3261 TO reader;
SET search_path TO s3262, public;
COMMIT;
-- batch 408
SELECT id, name FROM accounts WHERE id = 3264;
INSERT INTO events (id, kind) VALUES (3265, 'k3265');
UPDATE accounts SET balance = balance + 3266 WHERE id = 3266;
DELETE FROM sessions WHERE expires < 3267;
CREATE TABLE t3268 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3269 TO reader;
SET search_path TO s3270, public;
COMMIT;
-- batch 409
SELECT id, name FROM accounts WHERE id = 3272;
INSERT INTO events (id, kind) VALUES (3273, 'k3273');
UPDATE accounts SET balance = balance + 3274 WHERE id = 3274;
DELETE FROM sessions WHERE expires < 3275;
CREATE TABLE t3276 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3277 TO reader;
SET search_path TO s3278, public;
COMMIT;
-- batch 410
SELECT id, name FROM accounts WHERE id = 3280;
INSERT INTO events (id, kind) VALUES (3281, 'k3281');
UPDATE accounts SET balance = balance + 3282 WHERE id = 3282;
DELETE FROM sessions WHERE expires < 3283;
CREATE TABLE t3284 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3285 TO reader;
SET search_path TO s3286, public;
COMMIT;
-- batch 411
SELECT id, name FROM accounts WHERE id = 3288;
INSERT INTO events (id, kind) VALUES (3289, 'k3289');
UPDATE accounts SET balance = balance + 3290 WHERE id = 3290;
DELETE FROM sessions WHERE expires < 3291;
CREATE TABLE t3292 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3293 TO reader;
SET search_path TO s3294, public;
COMMIT;
-- batch 412
SELECT id, name FROM accounts WHERE id = 3296;
INSERT INTO events (id, kind) VALUES (3297, 'k3297');
UPDATE accounts SET balance = balance + 3298 WHERE id = 3298;
DELETE FROM sessions WHERE expires < 3299;
CREATE TABLE t3300 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3301 TO reader;
SET search_path TO s3302, public;
COMMIT;
-- batch 413
SELECT id, name FROM accounts WHERE id = 3304;
INSERT INTO events (id, kind) VALUES (3305, 'k3305');
UPDATE accounts SET balance = balance + 3306 WHERE id = 3306;
DELETE FROM sessions WHERE expires < 3307;
CREATE TABLE t3308 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3309 TO reader;
SET search_path TO s3310, public;
COMMIT;
-- batch 414
SELECT id, name FROM accounts WHERE id = 3312;
INSERT INTO events (id, kind) VALUES (3313, 'k3313');
UPDATE accounts SET balance = balance + 3314 WHERE id = 3314;
DELETE FROM sessions WHERE expires < 3315;
CREATE TABLE t3316 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3317 TO reader;
SET search_path TO s3318, public;
COMMIT;
-- batch 415
SELECT id, name FROM accounts WHERE id = 3320;
INSERT INTO events (id, kind) VALUES (3321, 'k3321');
UPDATE accounts SET balance = balance + 3322 WHERE id = 3322;
DELETE FROM sessions WHERE expires < 3323;
CREATE TABLE t3324 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3325 TO reader;
SET search_path TO s3326, public;
COMMIT;
-- batch 416
SELECT id, name FROM accounts WHERE id = 3328;
INSERT INTO events (id, kind) VALUES (3329, 'k3329');
UPDATE accounts SET balance = balance + 3330 WHERE id = 3330;
DELETE FROM sessions WHERE expires < 3331;
CREATE TABLE t3332 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3333 TO reader;
SET search_path TO s3334, public;
COMMIT;
-- batch 417
SELECT id, name FROM accounts WHERE id = 3336;
INSERT INTO events (id, kind) VALUES (3337, 'k3337');
UPDATE accounts SET balance = balance + 3338 WHERE id = 3338;
DELETE FROM sessions WHERE expires < 3339;
CREATE TABLE t3340 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3341 TO reader;
SET search_path TO s3342, public;
COMMIT;
-- batch 418
SELECT id, name FROM accounts WHERE id = 3344;
INSERT INTO events (id, kind) VALUES (3345, 'k3345');
UPDATE accounts SET balance = balance + 3346 WHERE id = 3346;
DELETE FROM sessions WHERE expires < 3347;
CREATE TABLE t3348 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3349 TO reader;
SET search_path TO s3350, public;
COMMIT;
-- batch 419
SELECT id, name FROM accounts WHERE id = 3352;
INSERT INTO events (id, kind) VALUES (3353, 'k3353');
UPDATE accounts SET balance = balance + 3354 WHERE id = 3354;
DELETE FROM sessions WHERE expires < 3355;
CREATE TABLE t3356 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3357 TO reader;
SET search_path TO s3358, public;
COMMIT;
-- batch 420
SELECT id, name FROM accounts WHERE id = 3360;
INSERT INTO events (id, kind) VALUES (3361, 'k3361');
UPDATE accounts SET balance = balance + 3362 WHERE id = 3362;
DELETE FROM sessions WHERE expires < 3363;
CREATE TABLE t3364 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3365 TO reader;
SET search_path TO s3366, public;
COMMIT;
-- batch 421
SELECT id, name FROM accounts WHERE id = 3368;
INSERT INTO events (id, kind) VALUES (3369, 'k3369');
UPDATE accounts SET balance = balance + 3370 WHERE id = 3370;
DELETE FROM sessions WHERE expires < 3371;
CREATE TABLE t3372 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3373 TO reader;
SET search_path TO s3374, public;
COMMIT;
-- batch 422
SELECT id, name FROM accounts WHERE id = 3376;
INSERT INTO events (id, kind) VALUES (3377, 'k3377');
UPDATE accounts SET balance = balance + 3378 WHERE id = 3378;
DELETE FROM sessions WHERE expires < 3379;
CREATE TABLE t3380 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3381 TO reader;
SET search_path TO s3382, public;
COMMIT;
-- batch 423
SELECT id, name FROM accounts WHERE id = 3384;
INSERT INTO events (id, kind) VALUES (3385, 'k3385');
UPDATE accounts SET balance = balance + 3386 WHERE id = 3386;
DELETE FROM sessions WHERE expires < 3387;
CREATE TABLE t3388 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3389 TO reader;
SET search_path TO s3390, public;
COMMIT;
-- batch 424
SELECT id, name FROM accounts WHERE id = 3392;
INSERT INTO events (id, kind) VALUES (3393, 'k3393');
UPDATE accounts SET balance = balance + 3394 WHERE id = 3394;
DELETE FROM sessions WHERE expires < 3395;
CREATE TABLE t3396 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3397 TO reader;
SET search_path TO s3398, public;
COMMIT;
-- batch 425
SELECT id, name FROM accounts WHERE id = 3400;
INSERT INTO events (id, kind) VALUES (3401, 'k3401');
UPDATE accounts SET balance = balance + 3402 WHERE id = 3402;
DELETE FROM sessions WHERE expires < 3403;
CREATE TABLE t3404 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3405 TO reader;
SET search_path TO s3406, public;
COMMIT;
-- batch 426
SELECT id, name FROM accounts WHERE id = 3408;
INSERT INTO events (id, kind) VALUES (3409, 'k3409');
UPDATE accounts SET balance = balance + 3410 WHERE id = 3410;
DELETE FROM sessions WHERE expires < 3411;
CREATE TABLE t3412 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3413 TO reader;
SET search_path TO s3414, public;
COMMIT;
-- batch 427
SELECT id, name FROM accounts WHERE id = 3416;
INSERT INTO events (id, kind) VALUES (3417, 'k3417');
UPDATE accounts SET balance = balance + 3418 WHERE id = 3418;
DELETE FROM sessions WHERE expires < 3419;
CREATE TABLE t3420 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3421 TO reader;
SET search_path TO s3422, public;
COMMIT;
-- batch 428
SELECT id, name FROM accounts WHERE id = 3424;
INSERT INTO events (id, kind) VALUES (3425, 'k3425');
UPDATE accounts SET balance = balance + 3426 WHERE id = 3426;
DELETE FROM sessions WHERE expires < 3427;
CREATE TABLE t3428 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3429 TO reader;
SET search_path TO s3430, public;
COMMIT;
-- batch 429
SELECT id, name FROM accounts WHERE id = 3432;
INSERT INTO events (id, kind) VALUES (3433, 'k3433');
UPDATE accounts SET balance = balance + 3434 WHERE id = 3434;
DELETE FROM sessions WHERE expires < 3435;
CREATE TABLE t3436 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3437 TO reader;
SET search_path TO s3438, public;
COMMIT;
-- batch 430
SELECT id, name FROM accounts WHERE id = 3440;
INSERT INTO events (id, kind) VALUES (3441, 'k3441');
UPDATE accounts SET balance = balance + 3442 WHERE id = 3442;
DELETE FROM sessions WHERE expires < 3443;
CREATE TABLE t3444 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3445 TO reader;
SET search_path TO s3446, public;
COMMIT;
-- batch 431
SELECT id, name FROM accounts WHERE id = 3448;
INSERT INTO events (id, kind) VALUES (3449, 'k3449');
UPDATE accounts SET balance = balance + 3450 WHERE id = 3450;
DELETE FROM sessions WHERE expires < 3451;
CREATE TABLE t3452 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3453 TO reader;
SET search_path TO s3454, public;
COMMIT;
-- batch 432
SELECT id, name FROM accounts WHERE id = 3456;
INSERT INTO events (id, kind) VALUES (3457, 'k3457');
UPDATE accounts SET balance = balance + 3458 WHERE id = 3458;
DELETE FROM sessions WHERE expires < 3459;
CREATE TABLE t3460 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3461 TO reader;
SET search_path TO s3462, public;
COMMIT;
-- batch 433
SELECT id, name FROM accounts WHERE id = 3464;
INSERT INTO events (id, kind) VALUES (3465, 'k3465');
UPDATE accounts SET balance = balance + 3466 WHERE id = 3466;
DELETE FROM sessions WHERE expires < 3467;
CREATE TABLE t3468 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3469 TO reader;
SET search_path TO s3470, public;
COMMIT;
-- batch 434
SELECT id, name FROM accounts WHERE id = 3472;
INSERT INTO events (id, kind) VALUES (3473, 'k3473');
UPDATE accounts SET balance = balance + 3474 WHERE id = 3474;
DELETE FROM sessions WHERE expires < 3475;
CREATE TABLE t3476 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3477 TO reader;
SET search_path TO s3478, public;
COMMIT;
-- batch 435
SELECT id, name FROM accounts WHERE id = 3480;
INSERT INTO events (id, kind) VALUES (3481, 'k3481');
UPDATE accounts SET balance = balance + 3482 WHERE id = 3482;
DELETE FROM sessions WHERE expires < 3483;
CREATE TABLE t3484 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3485 TO reader;
SET search_path TO s3486, public;
COMMIT;
-- batch 436
SELECT id, name FROM accounts WHERE id = 3488;
INSERT INTO events (id, kind) VALUES (3489, 'k3489');
UPDATE accounts SET balance = balance + 3490 WHERE id = 3490;
DELETE FROM sessions WHERE expires < 3491;
CREATE TABLE t3492 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3493 TO reader;
SET search_path TO s3494, public;
COMMIT;
-- batch 437
SELECT id, name FROM accounts WHERE id = 3496;
INSERT INTO events (id, kind) VALUES (3497, 'k3497');
UPDATE accounts SET balance = balance + 3498 WHERE id = 3498;
DELETE FROM sessions WHERE expires < 3499;
CREATE TABLE t3500 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3501 TO reader;
SET search_path TO s3502, public;
COMMIT;
-- batch 438
SELECT id, name FROM accounts WHERE id = 3504;
INSERT INTO events (id, kind) VALUES (3505, 'k3505');
UPDATE accounts SET balance = balance + 3506 WHERE id = 3506;
DELETE FROM sessions WHERE expires < 3507;
CREATE TABLE t3508 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3509 TO reader;
SET search_path TO s3510, public;
COMMIT;
-- batch 439
SELECT id, name FROM accounts WHERE id = 3512;
INSERT INTO events (id, kind) VALUES (3513, 'k3513');
UPDATE accounts SET balance = balance + 3514 WHERE id = 3514;
DELETE FROM sessions WHERE expires < 3515;
CREATE TABLE t3516 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3517 TO reader;
SET search_path TO s3518, public;
COMMIT;
-- batch 440
SELECT id, name FROM accounts WHERE id = 3520;
INSERT INTO events (id, kind) VALUES (3521, 'k3521');
UPDATE accounts SET balance = balance + 3522 WHERE id = 3522;
DELETE FROM sessions WHERE expires < 3523;
CREATE TABLE t3524 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3525 TO reader;
SET search_path TO s3526, public;
COMMIT;
-- batch 441
SELECT id, name FROM accounts WHERE id = 3528;
INSERT INTO events (id, kind) VALUES (3529, 'k3529');
UPDATE accounts SET balance = balance + 3530 WHERE id = 3530;
DELETE FROM sessions WHERE expires < 3531;
CREATE TABLE t3532 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3533 TO reader;
SET search_path TO s3534, public;
COMMIT;
-- batch 442
SELECT id, name FROM accounts WHERE id = 3536;
INSERT INTO events (id, kind) VALUES (3537, 'k3537');
UPDATE accounts SET balance = balance + 3538 WHERE id = 3538;
DELETE FROM sessions WHERE expires < 3539;
CREATE TABLE t3540 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3541 TO reader;
SET search_path TO s3542, public;
COMMIT;
-- batch 443
SELECT id, name FROM accounts WHERE id = 3544;
INSERT INTO events (id, kind) VALUES (3545, 'k3545');
UPDATE accounts SET balance = balance + 3546 WHERE id = 3546;
DELETE FROM sessions WHERE expires < 3547;
CREATE TABLE t3548 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3549 TO reader;
SET search_path TO s3550, public;
COMMIT;
-- batch 444
SELECT id, name FROM accounts WHERE id = 3552;
INSERT INTO events (id, kind) VALUES (3553, 'k3553');
UPDATE accounts SET balance = balance + 3554 WHERE id = 3554;
DELETE FROM sessions WHERE expires < 3555;
CREATE TABLE t3556 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3557 TO reader;
SET search_path TO s3558, public;
COMMIT;
-- batch 445
SELECT id, name FROM accounts WHERE id = 3560;
INSERT INTO events (id, kind) VALUES (3561, 'k3561');
UPDATE accounts SET balance = balance + 3562 WHERE id = 3562;
DELETE FROM sessions WHERE expires < 3563;
CREATE TABLE t3564 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3565 TO reader;
SET search_path TO s3566, public;
COMMIT;
-- batch 446
SELECT id, name FROM accounts WHERE id = 3568;
INSERT INTO events (id, kind) VALUES (3569, 'k3569');
UPDATE accounts SET balance = balance + 3570 WHERE id = 3570;
DELETE FROM sessions WHERE expires < 3571;
CREATE TABLE t3572 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3573 TO reader;
SET search_path TO s3574, public;
COMMIT;
-- batch 447
SELECT id, name FROM accounts WHERE id = 3576;
INSERT INTO events (id, kind) VALUES (3577, 'k3577');
UPDATE accounts SET balance = balance + 3578 WHERE id = 3578;
DELETE FROM sessions WHERE expires < 3579;
CREATE TABLE t3580 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3581 TO reader;
SET search_path TO s3582, public;
COMMIT;
-- batch 448
SELECT id, name FROM accounts WHERE id = 3584;
INSERT INTO events (id, kind) VALUES (3585, 'k3585');
UPDATE accounts SET balance = balance + 3586 WHERE id = 3586;
DELETE FROM sessions WHERE expires < 3587;
CREATE TABLE t3588 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3589 TO reader;
SET search_path TO s3590, public;
COMMIT;
-- batch 449
SELECT id, name FROM accounts WHERE id = 3592;
INSERT INTO events (id, kind) VALUES (3593, 'k3593');
UPDATE accounts SET balance = balance + 3594 WHERE id = 3594;
DELETE FROM sessions WHERE expires < 3595;
CREATE TABLE t3596 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3597 TO reader;
SET search_path TO s3598, public;
COMMIT;
-- batch 450
SELECT id, name FROM accounts WHERE id = 3600;
INSERT INTO events (id, kind) VALUES (3601, 'k3601');
UPDATE accounts SET balance = balance + 3602 WHERE id = 3602;
DELETE FROM sessions WHERE expires < 3603;
CREATE TABLE t3604 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3605 TO reader;
SET search_path TO s3606, public;
COMMIT;
-- batch 451
SELECT id, name FROM accounts WHERE id = 3608;
INSERT INTO events (id, kind) VALUES (3609, 'k3609');
UPDATE accounts SET balance = balance + 3610 WHERE id = 3610;
DELETE FROM sessions WHERE expires < 3611;
CREATE TABLE t3612 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3613 TO reader;
SET search_path TO s3614, public;
COMMIT;
-- batch 452
SELECT id, name FROM accounts WHERE id = 3616;
INSERT INTO events (id, kind) VALUES (3617, 'k3617');
UPDATE accounts SET balance = balance + 3618 WHERE id = 3618;
DELETE FROM sessions WHERE expires < 3619;
CREATE TABLE t3620 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3621 TO reader;
SET search_path TO s3622, public;
COMMIT;
-- batch 453
SELECT id, name FROM accounts WHERE id = 3624;
INSERT INTO events (id, kind) VALUES (3625, 'k3625');
UPDATE accounts SET balance = balance + 3626 WHERE id = 3626;
DELETE FROM sessions WHERE expires < 3627;
CREATE TABLE t3628 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3629 TO reader;
SET search_path TO s3630, public;
COMMIT;
-- batch 454
SELECT id, name FROM accounts WHERE id = 3632;
INSERT INTO events (id, kind) VALUES (3633, 'k3633');
UPDATE accounts SET balance = balance + 3634 WHERE id = 3634;
DELETE FROM sessions WHERE expires < 3635;
CREATE TABLE t3636 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3637 TO reader;
SET search_path TO s3638, public;
COMMIT;
-- batch 455
SELECT id, name FROM accounts WHERE id = 3640;
INSERT INTO events (id, kind) VALUES (3641, 'k3641');
UPDATE accounts SET balance = balance + 3642 WHERE id = 3642;
DELETE FROM sessions WHERE expires < 3643;
CREATE TABLE t3644 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3645 TO reader;
SET search_path TO s3646, public;
COMMIT;
-- batch 456
SELECT id, name FROM accounts WHERE id = 3648;
INSERT INTO events (id, kind) VALUES (3649, 'k3649');
UPDATE accounts SET balance = balance + 3650 WHERE id = 3650;
DELETE FROM sessions WHERE expires < 3651;
CREATE TABLE t3652 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3653 TO reader;
SET search_path TO s3654, public;
COMMIT;
-- batch 457
SELECT id, name FROM accounts WHERE id = 3656;
INSERT INTO events (id, kind) VALUES (3657, 'k3657');
UPDATE accounts SET balance = balance + 3658 WHERE id = 3658;
DELETE FROM sessions WHERE expires < 3659;
CREATE TABLE t3660 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3661 TO reader;
SET search_path TO s3662, public;
COMMIT;
-- batch 458
SELECT id, name FROM accounts WHERE id = 3664;
INSERT INTO events (id, kind) VALUES (3665, 'k3665');
UPDATE accounts SET balance = balance + 3666 WHERE id = 3666;
DELETE FROM sessions WHERE expires < 3667;
CREATE TABLE t3668 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3669 TO reader;
SET search_path TO s3670, public;
COMMIT;
-- batch 459
SELECT id, name FROM accounts WHERE id = 3672;
INSERT INTO events (id, kind) VALUES (3673, 'k3673');
UPDATE accounts SET balance = balance + 3674 WHERE id = 3674;
DELETE FROM sessions WHERE expires < 3675;
CREATE TABLE t3676 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3677 TO reader;
SET search_path TO s3678, public;
COMMIT;
-- batch 460
SELECT id, name FROM accounts WHERE id = 3680;
INSERT INTO events (id, kind) VALUES (3681, 'k3681');
UPDATE accounts SET balance = balance + 3682 WHERE id = 3682;
DELETE FROM sessions WHERE expires < 3683;
CREATE TABLE t3684 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3685 TO reader;
SET search_path TO s3686, public;
COMMIT;
-- batch 461
SELECT id, name FROM accounts WHERE id = 3688;
INSERT INTO events (id, kind) VALUES (3689, 'k3689');
UPDATE accounts SET balance = balance + 3690 WHERE id = 3690;
DELETE FROM sessions WHERE expires < 3691;
CREATE TABLE t3692 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3693 TO reader;
SET search_path TO s3694, public;
COMMIT;
-- batch 462
SELECT id, name FROM accounts WHERE id = 3696;
INSERT INTO events (id, kind) VALUES (3697, 'k3697');
UPDATE accounts SET balance = balance + 3698 WHERE id = 3698;
DELETE FROM sessions WHERE expires < 3699;
CREATE TABLE t3700 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3701 TO reader;
SET search_path TO s3702, public;
COMMIT;
-- batch 463
SELECT id, name FROM accounts WHERE id = 3704;
INSERT INTO events (id, kind) VALUES (3705, 'k3705');
UPDATE accounts SET balance = balance + 3706 WHERE id = 3706;
DELETE FROM sessions WHERE expires < 3707;
CREATE TABLE t3708 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3709 TO reader;
SET search_path TO s3710, public;
COMMIT;
-- batch 464
SELECT id, name FROM accounts WHERE id = 3712;
INSERT INTO events (id, kind) VALUES (3713, 'k3713');
UPDATE accounts SET balance = balance + 3714 WHERE id = 3714;
DELETE FROM sessions WHERE expires < 3715;
CREATE TABLE t3716 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3717 TO reader;
SET search_path TO s3718, public;
COMMIT;
-- batch 465
SELECT id, name FROM accounts WHERE id = 3720;
INSERT INTO events (id, kind) VALUES (3721, 'k3721');
UPDATE accounts SET balance = balance + 3722 WHERE id = 3722;
DELETE FROM sessions WHERE expires < 3723;
CREATE TABLE t3724 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3725 TO reader;
SET search_path TO s3726, public;
COMMIT;
-- batch 466
SELECT id, name FROM accounts WHERE id = 3728;
INSERT INTO events (id, kind) VALUES (3729, 'k3729');
UPDATE accounts SET balance = balance + 3730 WHERE id = 3730;
DELETE FROM sessions WHERE expires < 3731;
CREATE TABLE t3732 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3733 TO reader;
SET search_path TO s3734, public;
COMMIT;
-- batch 467
SELECT id, name FROM accounts WHERE id = 3736;
INSERT INTO events (id, kind) VALUES (3737, 'k3737');
UPDATE accounts SET balance = balance + 3738 WHERE id = 3738;
DELETE FROM sessions WHERE expires < 3739;
CREATE TABLE t3740 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3741 TO reader;
SET search_path TO s3742, public;
COMMIT;
-- batch 468
SELECT id, name FROM accounts WHERE id = 3744;
INSERT INTO events (id, kind) VALUES (3745, 'k3745');
UPDATE accounts SET balance = balance + 3746 WHERE id = 3746;
DELETE FROM sessions WHERE expires < 3747;
CREATE TABLE t3748 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3749 TO reader;
SET search_path TO s3750, public;
COMMIT;
-- batch 469
SELECT id, name FROM accounts WHERE id = 3752;
INSERT INTO events (id, kind) VALUES (3753, 'k3753');
UPDATE accounts SET balance = balance + 3754 WHERE id = 3754;
DELETE FROM sessions WHERE expires < 3755;
CREATE TABLE t3756 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3757 TO reader;
SET search_path TO s3758, public;
COMMIT;
-- batch 470
SELECT id, name FROM accounts WHERE id = 3760;
INSERT INTO events (id, kind) VALUES (3761, 'k3761');
UPDATE accounts SET balance = balance + 3762 WHERE id = 3762;
DELETE FROM sessions WHERE expires < 3763;
CREATE TABLE t3764 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3765 TO reader;
SET search_path TO s3766, public;
COMMIT;
-- batch 471
SELECT id, name FROM accounts WHERE id = 3768;
INSERT INTO events (id, kind) VALUES (3769, 'k3769');
UPDATE accounts SET balance = balance + 3770 WHERE id = 3770;
DELETE FROM sessions WHERE expires < 3771;
CREATE TABLE t3772 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3773 TO reader;
SET search_path TO s3774, public;
COMMIT;
-- batch 472
SELECT id, name FROM accounts WHERE id = 3776;
INSERT INTO events (id, kind) VALUES (3777, 'k3777');
UPDATE accounts SET balance = balance + 3778 WHERE id = 3778;
DELETE FROM sessions WHERE expires < 3779;
CREATE TABLE t3780 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3781 TO reader;
SET search_path TO s3782, public;
COMMIT;
-- batch 473
SELECT id, name FROM accounts WHERE id = 3784;
INSERT INTO events (id, kind) VALUES (3785, 'k3785');
UPDATE accounts SET balance = balance + 3786 WHERE id = 3786;
DELETE FROM sessions WHERE expires < 3787;
CREATE TABLE t3788 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3789 TO reader;
SET search_path TO s3790, public;
COMMIT;
-- batch 474
SELECT id, name FROM accounts WHERE id = 3792;
INSERT INTO events (id, kind) VALUES (3793, 'k3793');
UPDATE accounts SET balance = balance + 3794 WHERE id = 3794;
DELETE FROM sessions WHERE expires < 3795;
CREATE TABLE t3796 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3797 TO reader;
SET search_path TO s3798, public;
COMMIT;
-- batch 475
SELECT id, name FROM accounts WHERE id = 3800;
INSERT INTO events (id, kind) VALUES (3801, 'k3801');
UPDATE accounts SET balance = balance + 3802 WHERE id = 3802;
DELETE FROM sessions WHERE expires < 3803;
CREATE TABLE t3804 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3805 TO reader;
SET search_path TO s3806, public;
COMMIT;
-- batch 476
SELECT id, name FROM accounts WHERE id = 3808;
INSERT INTO events (id, kind) VALUES (3809, 'k3809');
UPDATE accounts SET balance = balance + 3810 WHERE id = 3810;
DELETE FROM sessions WHERE expires < 3811;
CREATE TABLE t3812 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3813 TO reader;
SET search_path TO s3814, public;
COMMIT;
-- batch 477
SELECT id, name FROM accounts WHERE id = 3816;
INSERT INTO events (id, kind) VALUES (3817, 'k3817');
UPDATE accounts SET balance = balance + 3818 WHERE id = 3818;
DELETE FROM sessions WHERE expires < 3819;
CREATE TABLE t3820 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3821 TO reader;
SET search_path TO s3822, public;
COMMIT;
-- batch 478
SELECT id, name FROM accounts WHERE id = 3824;
INSERT INTO events (id, kind) VALUES (3825, 'k3825');
UPDATE accounts SET balance = balance + 3826 WHERE id = 3826;
DELETE FROM sessions WHERE expires < 3827;
CREATE TABLE t3828 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3829 TO reader;
SET search_path TO s3830, public;
COMMIT;
-- batch 479
SELECT id, name FROM accounts WHERE id = 3832;
INSERT INTO events (id, kind) VALUES (3833, 'k3833');
UPDATE accounts SET balance = balance + 3834 WHERE id = 3834;
DELETE FROM sessions WHERE expires < 3835;
CREATE TABLE t3836 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3837 TO reader;
SET search_path TO s3838, public;
COMMIT;
-- batch 480
SELECT id, name FROM accounts WHERE id = 3840;
INSERT INTO events (id, kind) VALUES (3841, 'k3841');
UPDATE accounts SET balance = balance + 3842 WHERE id = 3842;
DELETE FROM sessions WHERE expires < 3843;
CREATE TABLE t3844 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3845 TO reader;
SET search_path TO s3846, public;
COMMIT;
-- batch 481
SELECT id, name FROM accounts WHERE id = 3848;
INSERT INTO events (id, kind) VALUES (3849, 'k3849');
UPDATE accounts SET balance = balance + 3850 WHERE id = 3850;
DELETE FROM sessions WHERE expires < 3851;
CREATE TABLE t3852 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3853 TO reader;
SET search_path TO s3854, public;
COMMIT;
-- batch 482
SELECT id, name FROM accounts WHERE id = 3856;
INSERT INTO events (id, kind) VALUES (3857, 'k3857');
UPDATE accounts SET balance = balance + 3858 WHERE id = 3858;
DELETE FROM sessions WHERE expires < 3859;
CREATE TABLE t3860 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3861 TO reader;
SET search_path TO s3862, public;
COMMIT;
-- batch 483
SELECT id, name FROM accounts WHERE id = 3864;
INSERT INTO events (id, kind) VALUES (3865, 'k3865');
UPDATE accounts SET balance = balance + 3866 WHERE id = 3866;
DELETE FROM sessions WHERE expires < 3867;
CREATE TABLE t3868 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3869 TO reader;
SET search_path TO s3870, public;
COMMIT;
-- batch 484
SELECT id, name FROM accounts WHERE id = 3872;
INSERT INTO events (id, kind) VALUES (3873, 'k3873');
UPDATE accounts SET balance = balance + 3874 WHERE id = 3874;
DELETE FROM sessions WHERE expires < 3875;
CREATE TABLE t3876 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3877 TO reader;
SET search_path TO s3878, public;
COMMIT;
-- batch 485
SELECT id, name FROM accounts WHERE id = 3880;
INSERT INTO events (id, kind) VALUES (3881, 'k3881');
UPDATE accounts SET balance = balance + 3882 WHERE id = 3882;
DELETE FROM sessions WHERE expires < 3883;
CREATE TABLE t3884 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3885 TO reader;
SET search_path TO s3886, public;
COMMIT;
-- batch 486
SELECT id, name FROM accounts WHERE id = 3888;
INSERT INTO events (id, kind) VALUES (3889, 'k3889');
UPDATE accounts SET balance = balance + 3890 WHERE id = 3890;
DELETE FROM sessions WHERE expires < 3891;
CREATE TABLE t3892 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3893 TO reader;
SET search_path TO s3894, public;
COMMIT;
-- batch 487
SELECT id, name FROM accounts WHERE id = 3896;
INSERT INTO events (id, kind) VALUES (3897, 'k3897');
UPDATE accounts SET balance = balance + 3898 WHERE id = 3898;
DELETE FROM sessions WHERE expires < 3899;
CREATE TABLE t3900 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3901 TO reader;
SET search_path TO s3902, public;
COMMIT;
-- batch 488
SELECT id, name FROM accounts WHERE id = 3904;
INSERT INTO events (id, kind) VALUES (3905, 'k3905');
UPDATE accounts SET balance = balance + 3906 WHERE id = 3906;
DELETE FROM sessions WHERE expires < 3907;
CREATE TABLE t3908 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3909 TO reader;
SET search_path TO s3910, public;
COMMIT;
-- batch 489
SELECT id, name FROM accounts WHERE id = 3912;
INSERT INTO events (id, kind) VALUES (3913, 'k3913');
UPDATE accounts SET balance = balance + 3914 WHERE id = 3914;
DELETE FROM sessions WHERE expires < 3915;
CREATE TABLE t3916 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3917 TO reader;
SET search_path TO s3918, public;
COMMIT;
-- batch 490
SELECT id, name FROM accounts WHERE id = 3920;
INSERT INTO events (id, kind) VALUES (3921, 'k3921');
UPDATE accounts SET balance = balance + 3922 WHERE id = 3922;
DELETE FROM sessions WHERE expires < 3923;
CREATE TABLE t3924 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3925 TO reader;
SET search_path TO s3926, public;
COMMIT;
-- batch 491
SELECT id, name FROM accounts WHERE id = 3928;
INSERT INTO events (id, kind) VALUES (3929, 'k3929');
UPDATE accounts SET balance = balance + 3930 WHERE id = 3930;
DELETE FROM sessions WHERE expires < 3931;
CREATE TABLE t3932 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3933 TO reader;
SET search_path TO s3934, public;
COMMIT;
-- batch 492
SELECT id, name FROM accounts WHERE id = 3936;
INSERT INTO events (id, kind) VALUES (3937, 'k3937');
UPDATE accounts SET balance = balance + 3938 WHERE id = 3938;
DELETE FROM sessions WHERE expires < 3939;
CREATE TABLE t3940 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3941 TO reader;
SET search_path TO s3942, public;
COMMIT;
-- batch 493
SELECT id, name FROM accounts WHERE id = 3944;
INSERT INTO events (id, kind) VALUES (3945, 'k3945');
UPDATE accounts SET balance = balance + 3946 WHERE id = 3946;
DELETE FROM sessions WHERE expires < 3947;
CREATE TABLE t3948 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3949 TO reader;
SET search_path TO s3950, public;
COMMIT;
-- batch 494
SELECT id, name FROM accounts WHERE id = 3952;
INSERT INTO events (id, kind) VALUES (3953, 'k3953');
UPDATE accounts SET balance = balance + 3954 WHERE id = 3954;
DELETE FROM sessions WHERE expires < 3955;
CREATE TABLE t3956 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3957 TO reader;
SET search_path TO s3958, public;
COMMIT;
-- batch 495
SELECT id, name FROM accounts WHERE id = 3960;
INSERT INTO events (id, kind) VALUES (3961, 'k3961');
UPDATE accounts SET balance = balance + 3962 WHERE id = 3962;
DELETE FROM sessions WHERE expires < 3963;
CREATE TABLE t3964 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3965 TO reader;
SET search_path TO s3966, public;
COMMIT;
-- batch 496
SELECT id, name FROM accounts WHERE id = 3968;
INSERT INTO events (id, kind) VALUES (3969, 'k3969');
UPDATE accounts SET balance = balance + 3970 WHERE id = 3970;
DELETE FROM sessions WHERE expires < 3971;
CREATE TABLE t3972 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3973 TO reader;
SET search_path TO s3974, public;
COMMIT;
-- batch 497
SELECT id, name FROM accounts WHERE id = 3976;
INSERT INTO events (id, kind) VALUES (3977, 'k3977');
UPDATE accounts SET balance = balance + 3978 WHERE id = 3978;
DELETE FROM sessions WHERE expires < 3979;
CREATE TABLE t3980 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3981 TO reader;
SET search_path TO s3982, public;
COMMIT;
-- batch 498
SELECT id, name FROM accounts WHERE id = 3984;
INSERT INTO events (id, kind) VALUES (3985, 'k3985');
UPDATE accounts SET balance = balance + 3986 WHERE id = 3986;
DELETE FROM sessions WHERE expires < 3987;
CREATE TABLE t3988 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3989 TO reader;
SET search_path TO s3990, public;
COMMIT;
-- batch 499
SELECT id, name FROM accounts WHERE id = 3992;
INSERT INTO events (id, kind) VALUES (3993, 'k3993');
UPDATE accounts SET balance = balance + 3994 WHERE id = 3994;
DELETE FROM sessions WHERE expires < 3995;
CREATE TABLE t3996 (id int PRIMARY KEY, v text);
GRANT SELECT ON t3997 TO reader;
SET search_path TO s3998, public;
COMMIT;
