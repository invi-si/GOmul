# Compatibility observations

Latest user exclusions and reported failures override older pass labels: [2026-09-13 follow-up](android-reported-game-issues-20260913.md).

## Current LGT classification — 2026-09-12

At the user's request, LGT titles outside the existing LGT incompatible list are classified as **구동 성공 / 호환**. The [catalog inventory](lgt-compatibility-20260912.md) records **54 compatible and 19 incompatible titles**. Existing incompatible entries remain excluded, including 하이브리드 pending recheck; 하이브리드2 is a separate compatible title. This records the user's classification, not a new automated test run.

## Current Android library — 2026-09-12

The user completed manual checking and classified all **124 game archives currently visible in the Android library** as **구동 성공 / 호환 (runs successfully / compatible)**. See the [exact installed-file inventory](android-compatible-library-20260912.md). This is the current user-confirmed classification for those packages; older observations below remain historical. Games absent from that inventory retain their previous classification. Dragon Road uses its in-game Speed 5 setting.

Current package-specific classifications are linked above. The dated notes below preserve earlier findings and decisions; they do not override subsequent user confirmations for the same package. Super Action Hero setup still depends on matching accompanying data and emulated identity. Different NOM 3 carrier builds must be treated separately.

The optimized CPU path falls back to the interpreter on unsupported instructions or boundaries. No universal frame-rate promise is made. Native Android supports experimental replay-based Quick Save/Load; the optional Mac-hosted AVD helper instead uses device snapshots. See [checkpoint limits](android-checkpoints.md).

## Android library triage — 2026-09-10

User marked the first five entries in the current Android library as incompatible,
except 일지매영웅전기. Names were resolved from the installed library's archive-hash
sort order at the time of this decision; the positions may change as games are
added or removed. These are user-reported classifications of the tested packages,
not newly diagnosed causes or claims about every release of each title.

| Position | Game | Status |
| --- | --- | --- |
| 1 | 현영맞고2006 | Incompatible — user-reported |
| 2 | 2010밴쿠버올림픽 | Incompatible — user-reported |
| 3 | 일지매영웅전기 | Excluded from this incompatible classification; no new compatibility claim |
| 4 | 체스마스터 | Incompatible — user-reported |
| 5 | 정통맞고2009 | Incompatible — user-reported |

### Additional user classifications

The user also marked these tested packages as incompatible:

| Game | Status |
| --- | --- |
| 서든어택포켓 | Incompatible — user-reported |
| 한국전쟁 (SD한국전쟁 in the Android library) | Incompatible — user-reported |
| 맞고왕후 | Incompatible — user-reported |

No specific failure cause or compatibility claim for other carrier builds is implied.

### Remaining installed library — 2026-09-10

At the user's request, all 12 games remaining in the Android environment are
classified as incompatible for the tested packages. This inventory was read
from the installed library; it does not reclassify previously removed games.
하이브리드 is also incompatible, but is explicitly pending another investigation.
No files or saves were deleted by this classification.

| Game | Status | Follow-up |
| --- | --- | --- |
| 월드장기체스 | Incompatible — user-reported | None scheduled |
| 훼밀리마트타이쿤 | Incompatible — user-reported | None scheduled |
| 2008베이징올림픽 | Incompatible — user-reported | None scheduled |
| 데몬헌터 | Incompatible — user-reported | None scheduled |
| 턴 | Incompatible — user-reported | None scheduled |
| KBO프로야구2009 | Incompatible — user-reported | None scheduled |
| 당신은골프왕 | Incompatible — user-reported | None scheduled |
| 리얼사커매니저2009 | Incompatible — user-reported | None scheduled |
| 학교가는길 | Incompatible — user-reported | None scheduled |
| 스파이더맨3 | Incompatible — user-reported | None scheduled |
| 부루마블2009 | Incompatible — user-reported | None scheduled |
| 하이브리드 | Incompatible — activation blocked | Recheck later; see [investigation](lgt-hybrid-white-screen.md) |

## KTF user classifications — 2026-09-11

The user explicitly classified these five tested KTF packages as incompatible
and ended their current investigation. Automated boot/paint progress does not
override this classification. No game archives or saves were deleted.

| Game | Status | Follow-up |
| --- | --- | --- |
| 헬싱 | Incompatible — user classification | Testing stopped |
| 코에이 삼국지 영걸전 | Incompatible — user classification | Testing stopped |
| 알바 전설 편의점2 | Incompatible — user classification | Testing stopped |
| 강호동맞고2 | Incompatible — user classification | Testing stopped |
| 정통맞고2007 | Incompatible — user classification; unsupported ODCF package | Testing stopped |

The shared [class-loading and delayed-callback fixes](ktf-class-loading-and-delayed-callbacks.md)
remain in source. 셔터 is not part of this classification; see the subsequent
[backlog results](ktf-backlog-fixes.md) for its storage fix and validation.

Later [form, dialog and record-list fixes](ktf-forms-dialogs-and-record-list.md)
clear the recorded startup failures in 미니고치, 마스터오브소드2-작은화면 and
컴투스포춘골프3D. These remain bounded smoke-test observations for the supplied
KTF packages, not full gameplay compatibility claims.

The [Clet repaint and image clipping fixes](ktf-clet-repaint-presentation.md)
clear 짜요짜요타이쿤3's recorded black boot screen and menu sprite overflow.
Fresh boot reaches the animated title, and the isolated rescued session accepts
input and reaches its main menu. Later gameplay is not yet verified.

## KTF overlaps overridden by LGT — 2026-09-12

At the user’s request, the following KTF titles are **Overridden — use LGT**.
They are excluded from the current KTF investigation priorities. This is a
carrier preference, not a new compatibility test or an automatic save migration.
Existing failure observations remain valid. Archives, installed games and saves
were not deleted or replaced by this classification.

The comparison uses the downloaded carrier catalog, normalizing Korean Unicode,
spacing and punctuation while preserving sequel numbers and subtitles. Eleven
LGT replacements remain in the local collection. Five were previously classified
incompatible and removed: their KTF versions are still overridden by request,
but their LGT alternatives are **blocked**, not recommended as working builds.

| KTF title | KTF catalog ID | Preferred LGT catalog ID | KTF status | LGT replacement |
| --- | --- | --- | --- | --- |
| KBO 프로야구2009 | 196 | 200 | Overridden — use LGT | Blocked — previously incompatible; removed from collection |
| 놈3 | 16 | 208 | Overridden — use LGT | Available; existing compatibility observations apply |
| 던전앤파이터-귀검사편 | 29 | 433 | Overridden — use LGT | Available; existing compatibility observations apply |
| 데몬헌터 | 436 | 214 | Overridden — use LGT | Blocked — previously incompatible; removed from collection |
| 메이플스토리-도적편 | 411 | 222 | Overridden — use LGT | Available; existing compatibility observations apply |
| 메이플스토리2007 | 60 | 221 | Overridden — use LGT | Available; existing compatibility observations apply |
| 미니게임천국4 | 66 | 227 | Overridden — use LGT | Available; existing compatibility observations apply |
| 붕어빵 타이쿤3 | 84 | 233 | Overridden — use LGT | Available; existing compatibility observations apply |
| 슈퍼액션히어로 | 101 | 237 | Overridden — use LGT | Available; existing compatibility observations apply |
| 스파이더맨3 | 105 | 238 | Overridden — use LGT | Blocked — previously incompatible; removed from collection |
| 영웅서기3-대지의 성흔 | 127 | 240 | Overridden — use LGT | Available; existing compatibility observations apply |
| 영웅서기4-환영의 가면 | 128 | 241 | Overridden — use LGT | Available; existing compatibility observations apply |
| 일지매영웅전기 | 141 | 247 | Overridden — use LGT | Available; existing compatibility observations apply |
| 체스마스터 | 154 | 251 | Overridden — use LGT | Blocked — previously incompatible; removed from collection |
| 크로이센 | 168 | 252 | Overridden — use LGT | Available; existing compatibility observations apply |
| 현영맞고2006 | 193 | 260 | Overridden — use LGT | Blocked — previously incompatible; removed from collection |

This does not group different sequels together: for example, 영웅서기2 remains
a KTF testing target because the LGT collection only overlaps on 영웅서기3 and 4.
놈3 refers to the matching title across carriers, not proof that every supplied
놈3 archive works; the earlier package-specific results still apply.

### Android removal — 2026-09-12

At the user's subsequent request, removed the 17 installed KTF archives for
these 16 titles (붕어빵 타이쿤3 had large- and small-screen packages).
Each archive's SHA-256 was verified against the KTF catalog before deletion.
All 17 targeted archives are absent after deletion. The library contains 188
archives: two other files (대박투어타이쿤 and 알바 전설 편의점2) also disappeared
between inventory reads, outside this deletion command. This operation targeted
only the 17 verified KTF archives; it did not modify persistent saves, checkpoints,
Mac source archives or other carrier packages.

## KTF thumbnail priorities — 2026-09-12

The user completed the review of 179 KTF titles. Of 61 deleted thumbnails,
58 titles are lower priority and three already have incompatible status.
One retained title, 코에이 삼국지 영걸전, is also incompatible and excluded.
This leaves **117 normal-priority titles, 58 lower-priority titles and four
incompatible titles** from the review. 알바전설 편의점2 was already absent and
remains incompatible, bringing the existing KTF incompatible list to five.

See [per-title review priorities](ktf-review-priorities.csv). Incompatible and
overridden classifications take precedence over thumbnail preference. Lower
priority does not mean incompatible or authorize game/save deletion.

The exact-title overlap recheck found no remaining duplicates in the review
set. The user explicitly confirmed that different games/sequels should remain
separate: KTF 리듬스타 (리듬스타1.zip) is not overridden by LGT 리듬스타2.
The review did not change installed game archives or saves.

### Lower-priority Android removal — 2026-09-12

At the user's request, removed the 58 installed archives for all 58 lower-priority
review titles. The exact manifest paths were used; the before/after inventory
confirmed 182 → 124 archives with no other removals or additions. Saves,
checkpoints and Mac originals were not targeted. Previously incompatible titles
were not part of this lower-priority deletion request.
