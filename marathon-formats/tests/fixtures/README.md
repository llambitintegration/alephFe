# Test Fixtures

This directory holds data files used by integration tests.

## Sample files (committed)

- `sample.mml` — A small MML configuration file for testing section parsing.
- `sample_plugin/Plugin.xml` — A sample plugin metadata file.

## Game data files (not committed)

Integration tests in `real_data_tests.rs` optionally use real Marathon data files.
These are copyrighted and **must not be committed**. Tests that require them will
be skipped automatically when the files are absent.

To run the full integration test suite, obtain the files from a legal copy of
the game and place them here:

| File | Source | Expected name |
|------|--------|---------------|
| Map file | Marathon 2 or Marathon Infinity | `Map` or `Map.sceA` |
| Shapes file | Marathon 2 or Marathon Infinity | `Shapes` |
| Sounds file | Marathon 2 or Marathon Infinity | `Sounds` |
| Physics file | Marathon 2 or Marathon Infinity | `Physics Model` |

### Where to get Marathon data files

Marathon 2: Durandal and Marathon Infinity are available as free downloads from
[Aleph One's website](https://alephone.lhowon.org/). Download any scenario
and copy the data files into this directory.
