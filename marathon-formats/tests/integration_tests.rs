use marathon_formats::tags::WadTag;
use marathon_formats::test_helpers::*;
use marathon_formats::wad::WadFile;

#[test]
fn test_integration_roundtrip_wad_with_map_geometry() {
    // Arrange: build a complete map entry with endpoints, lines, and a polygon
    let endpoints = MapDataBuilder::endpoints(&[(0, 0), (1024, 0), (1024, 1024), (0, 1024)]);
    let lines =
        MapDataBuilder::lines(&[(0, 1, 0, -1), (1, 2, 0, -1), (2, 3, 0, -1), (3, 0, 0, -1)]);
    let polygon = MapDataBuilder::polygon(4, &[0, 1, 2, 3], &[0, 1, 2, 3]);

    let wad_data = WadBuilder::new()
        .version(4)
        .file_name("Test Level")
        .add_entry(
            0,
            vec![
                TagData::new(WadTag::Endpoints, endpoints),
                TagData::new(WadTag::Lines, lines),
                TagData::new(WadTag::Polygons, polygon),
            ],
        )
        .build();

    // Act: parse the WAD
    let wad = WadFile::from_bytes(&wad_data).unwrap();

    // Assert: verify structure
    assert_eq!(wad.header.version, 4);
    assert_eq!(wad.header.file_name, "Test Level");
    assert_eq!(wad.entry_count(), 1);

    let entry = wad.entry(0).unwrap();
    assert_eq!(entry.all_tags().len(), 3);

    // Verify tag data sizes match expected struct sizes
    let ep_data = entry.get_tag_data(WadTag::Endpoints).unwrap();
    assert_eq!(ep_data.len(), 4 * 16, "4 endpoints * 16 bytes each");

    let ln_data = entry.get_tag_data(WadTag::Lines).unwrap();
    assert_eq!(ln_data.len(), 4 * 32, "4 lines * 32 bytes each");

    let poly_data = entry.get_tag_data(WadTag::Polygons).unwrap();
    assert_eq!(poly_data.len(), 128, "1 polygon * 128 bytes");
}

#[test]
fn test_integration_multi_level_wad() {
    // Arrange: build a WAD with multiple levels (entries)
    let level0_ep = MapDataBuilder::endpoints(&[(0, 0), (1024, 0), (0, 1024)]);
    let level1_ep = MapDataBuilder::endpoints(&[(0, 0), (2048, 0), (0, 2048)]);

    let wad_data = WadBuilder::new()
        .version(4)
        .file_name("Multi Level")
        .add_entry(0, vec![TagData::new(WadTag::Endpoints, level0_ep)])
        .add_entry(1, vec![TagData::new(WadTag::Endpoints, level1_ep)])
        .build();

    // Act
    let wad = WadFile::from_bytes(&wad_data).unwrap();

    // Assert
    assert_eq!(wad.entry_count(), 2);

    let e0 = wad.entry(0).unwrap();
    assert_eq!(e0.get_tag_data(WadTag::Endpoints).unwrap().len(), 3 * 16);

    let e1 = wad.entry(1).unwrap();
    assert_eq!(e1.get_tag_data(WadTag::Endpoints).unwrap().len(), 3 * 16);

    // Entries should not share tag data
    assert_ne!(
        e0.get_tag_data(WadTag::Endpoints).unwrap(),
        e1.get_tag_data(WadTag::Endpoints).unwrap(),
    );
}
