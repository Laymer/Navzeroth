# Third-party notices for the desktop application

## Fonts (SIL Open Font License 1.1)

Both families are bundled as **unmodified upstream files**. They are not subset,
converted, renamed or otherwise changed, so the OFL Reserved Font Name restrictions
for modified versions do not apply. The license texts ship beside the files in
`src/theme/fonts/` and in the Windows installer under `font-notices/`. The Windows
packaging workflow compares the installed notices with these sources byte for byte.

| Family | File in this repository | Upstream release | sha256 of the file |
| --- | --- | --- | --- |
| Inter 4.1 (variable) | `src/theme/fonts/InterVariable.woff2` | `https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip`, member `web/InterVariable.woff2` (archive sha256 `9883fdd4a49d4fb66bd8177ba6625ef9a64aa45899767dde3d36aa425756b11e`) | `693b77d4f32ee9b8bfc995589b5fad5e99adf2832738661f5402f9978429a8e3` |
| Cinzel (variable) | `src/theme/fonts/Cinzel-VariableFont_wght.ttf` | `https://raw.githubusercontent.com/google/fonts/3dd78844021e948ceb633d1dcee3f7885561b5d9/ofl/cinzel/Cinzel%5Bwght%5D.ttf` (google/fonts commit `3dd78844021e948ceb633d1dcee3f7885561b5d9`) | `f4d83d34d1f6c741193e4acf4b3dff9531e5a67b6aa65228d00a7db72a4e0f34` |

| License text | Source | sha256 |
| --- | --- | --- |
| `src/theme/fonts/OFL-Inter.txt` | `LICENSE.txt` in `Inter-4.1.zip` | `262481e844521b326f5ecd053e59b98c8b2da78c8ee1bdbb6e8174305e54935a` |
| `src/theme/fonts/OFL-Cinzel.txt` | `ofl/cinzel/OFL.txt` at the commit above | `f2b3029aba64c378bf0963b62945eee15e564fe4330b934c8f2eb058282b5e83` |

- Inter: Copyright (c) 2016 The Inter Project Authors (https://github.com/rsms/inter).
- Cinzel: Copyright 2020 The Cinzel Project Authors (https://github.com/NDISCOVER/Cinzel).

Verify with `sha256sum apps/desktop/src/theme/fonts/*`. The fonts are loaded from
the application bundle only (`src/theme/fonts.css`); nothing is fetched from a
font service at run time, which the application's Content-Security-Policy would
refuse in any case.

## Artwork

The citadel, ridge and treeline backdrop (`src/theme/Backdrop.tsx`) is original
vector art drawn for this project as inline SVG. It contains no raster image, no
game asset and no third-party artwork.

## Not affiliated

Azeroth Navigator is an independent project. It is not affiliated with or
endorsed by Blizzard Entertainment. World of Warcraft is a trademark of its owner.
