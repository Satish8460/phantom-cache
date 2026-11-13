
**Path 1: software compositing (--disable-gpu; renderer reported as ANGLE Microsoft Basic Render Driver; frame cadence capped at about 60 Hz, 16.7 ms).** Chrome/156.0.8078.79, 3 s scripted scroll at 1800 px/s, 5 runs per cell (median, then [min-max]).

| Viewport | Variant | Frames in 3 s | Long frames >17.7 ms | >34 ms | p95 frame ms | Max frame ms | Paint ms* | Raster ms* | Prepaint ms* | Layers |
|---|---|---|---|---|---|---|---|---|---|---|
| 1280x800 | a solid | 179 | 0 [0-0] | 0 | 16.8 | 16.8 [16.8-16.8] | 7.7 | 156.6 | 7.3 | n/a |
| 1280x800 | b blur12 x2 | 179 | 0 [0-0] | 0 | 16.8 | 16.8 [16.8-16.8] | 8.2 | 160.9 | 8.2 | n/a |
| 1280x800 | c blur12 x6 | 179 | 0 [0-0] | 0 | 16.7 | 16.8 [16.8-16.8] | 9.1 | 148.9 | 9.3 | n/a |
| 1280x800 | d large (rail+hdr+toolbar) | 179 | 0 [0-0] | 0 | 16.7 | 16.8 [16.8-16.8] | 11.2 | 177.8 | 10.3 | n/a |
| 1280x800 | d32 large, blur 32 | 179 | 0 [0-2] | 0 | 16.7 | 16.8 [16.8-33.4] | 8.2 | 131.6 | 7.7 | n/a |
| 1280x800 | e = d + animation under glass | 178 | 0 [0-0] | 0 | 16.8 | 16.8 [16.8-16.8] | 6.4 | 146.8 | 65.2 | n/a |
| 1280x800 | f 3D tilt x6 | 179 | 0 [0-0] | 0 | 16.7 | 16.8 [16.8-16.8] | 244.8 | 2405.4 | 87.9 | n/a |
| 1280x800 | g blurred scrim + glass dialog | 173 | 7 [0-11] | 0 | 16.8 | 33.4 [16.8-33.4] | 8.6 | 161.4 | 7.2 | n/a |
| 1280x800 | h ANTI: blur on 90 panels | 150 | 29 [3-32] | 0 | 33.4 | 33.4 [33.4-49.9] | 1.7 | 200.4 | 6.6 | n/a |
| 390x844, 4x CPU | a solid | 179 | 0 [0-0] | 0 | 16.8 | 16.8 [16.8-16.8] | 15.5 | 106.7 | 24 | n/a |
| 390x844, 4x CPU | b blur12 x2 | 179 | 0 [0-0] | 0 | 16.7 | 16.8 [16.8-16.8] | 15.2 | 59.6 | 29.7 | n/a |
| 390x844, 4x CPU | c blur12 x6 | 179 | 0 [0-0] | 0 | 16.7 | 16.8 [16.8-16.8] | 13.9 | 74.3 | 30.6 | n/a |
| 390x844, 4x CPU | d large (rail+hdr+toolbar) | 179 | 0 [0-0] | 0 | 16.7 | 16.8 [16.8-16.8] | 20.8 | 50.9 | 50.8 | n/a |
| 390x844, 4x CPU | d32 large, blur 32 | 179 | 0 [0-0] | 0 | 16.8 | 16.8 [16.8-16.8] | 25.3 | 50.2 | 53.9 | n/a |
| 390x844, 4x CPU | e = d + animation under glass | 178 | 0 [0-0] | 0 | 16.7 | 16.8 [16.8-16.8] | 19.6 | 54.3 | 488.6 | n/a |
| 390x844, 4x CPU | f 3D tilt x6 | 159 | 20 [7-34] | 2 | 33.3 | 50.1 [49.9-66.6] | 1435.6 | 1025.1 | 384.7 | n/a |
| 390x844, 4x CPU | g blurred scrim + glass dialog | 179 | 0 [0-0] | 0 | 16.7 | 16.8 [16.8-16.8] | 12.7 | 61.8 | 27.4 | n/a |
| 390x844, 4x CPU | h ANTI: blur on 90 panels | 179 | 0 [0-0] | 0 | 16.7 | 16.8 [16.8-16.8] | 5.5 | 72.7 | 28.1 | n/a |

**Path 2: hardware (no --disable-gpu; renderer reported as ANGLE Intel UHD Graphics 0x0000A78B; frame cadence about 240 Hz, 4.2 ms).** Chrome/156.0.8078.79, 3 s scripted scroll at 1800 px/s, 5 runs per cell (median, then [min-max]).

| Viewport | Variant | Frames in 3 s | Long frames >17.7 ms | >34 ms | p95 frame ms | Max frame ms | Paint ms* | Raster ms* | Prepaint ms* | Layers |
|---|---|---|---|---|---|---|---|---|---|---|
| 1280x800 | a solid | 719 | 0 [0-0] | 0 | 4.3 | 4.5 [4.4-8.2] | 9.1 | 2.5 | 21.4 | n/a |
| 1280x800 | b blur12 x2 | 719 | 0 [0-0] | 0 | 4.3 | 4.9 [4.4-12.5] | 6.7 | 1.9 | 21.5 | 10 |
| 1280x800 | c blur12 x6 | 718 | 0 [0-1] | 0 | 4.3 | 4.4 [4.4-24.9] | 7 | 2.1 | 24.1 | n/a |
| 1280x800 | d large (rail+hdr+toolbar) | 719 | 0 [0-0] | 0 | 4.3 | 4.5 [4.4-4.7] | 6.6 | 1.8 | 18.8 | 10 |
| 1280x800 | d32 large, blur 32 | 718 | 0 [0-0] | 0 | 4.3 | 4.5 [4.4-8.4] | 7.4 | 2 | 24 | 10 |
| 1280x800 | e = d + animation under glass | 718 | 0 [0-0] | 0 | 4.3 | 4.4 [4.4-4.5] | 8.4 | 3.3 | 287.7 | 23 |
| 1280x800 | f 3D tilt x6 | 717 | 0 [0-2] | 0 | 4.3 | 8.2 [4.9-20.9] | 792.4 | 28 | 225.7 | 64 |
| 1280x800 | g blurred scrim + glass dialog | 718 | 0 [0-5] | 0 | 4.3 | 4.8 [4.5-25.2] | 12 | 3 | 27.9 | n/a |
| 1280x800 | h ANTI: blur on 90 panels | 718 | 0 [0-1] | 0 | 4.3 | 4.6 [4.5-29.2] | 1.9 | 2.5 | 24.8 | n/a |
| 390x844, 4x CPU | a solid | 719 | 0 [0-0] | 0 | 4.3 | 4.5 [4.4-8.3] | 13.9 | 1.3 | 71.8 | n/a |
| 390x844, 4x CPU | b blur12 x2 | 718 | 0 [0-1] | 0 | 4.3 | 8.2 [4.5-20.8] | 18.8 | 1.4 | 124.7 | n/a |
| 390x844, 4x CPU | c blur12 x6 | 719 | 0 [0-0] | 0 | 4.3 | 4.5 [4.4-8.4] | 16.6 | 1.2 | 129.7 | 11 |
| 390x844, 4x CPU | d large (rail+hdr+toolbar) | 718 | 0 [0-0] | 0 | 4.3 | 4.5 [4.4-8.3] | 17.6 | 1.3 | 111.8 | 10 |
| 390x844, 4x CPU | d32 large, blur 32 | 719 | 0 [0-0] | 0 | 4.3 | 4.4 [4.3-5.9] | 18.5 | 1.6 | 134 | 10 |
| 390x844, 4x CPU | e = d + animation under glass | 680 | 0 [0-1] | 0 | 4.4 | 12.6 [4.4-20.8] | 19.2 | 1.6 | 1061.2 | n/a |
| 390x844, 4x CPU | f 3D tilt x6 | 295 | 29 [16-40] | 2 | 25 | 41.6 [33.5-58.5] | 1546.2 | 16.3 | 499.4 | n/a |
| 390x844, 4x CPU | g blurred scrim + glass dialog | 705 | 1 [0-1] | 0 | 4.3 | 20.8 [16.6-20.9] | 14.9 | 1.5 | 104.2 | n/a |
| 390x844, 4x CPU | h ANTI: blur on 90 panels | 718 | 0 [0-2] | 0 | 4.3 | 4.5 [4.4-29.2] | 6.5 | 2.4 | 164.3 | n/a |
