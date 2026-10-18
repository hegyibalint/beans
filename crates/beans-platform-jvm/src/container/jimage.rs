use std::{
    borrow::Cow,
    io::{Read, Seek, SeekFrom},
};

use beans_core::location::{Location, LocationEntry};

use crate::model::class::Class;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Endian {
    Little,
    Big,
}

impl Endian {
    fn read_u32(self, reader: &mut impl Read) -> Result<u32, String> {
        let mut bytes = [0; 4];
        reader
            .read_exact(&mut bytes)
            .map_err(|error| error.to_string())?;
        Ok(self.decode(bytes))
    }

    fn decode(self, bytes: [u8; 4]) -> u32 {
        match self {
            Self::Little => u32::from_le_bytes(bytes),
            Self::Big => u32::from_be_bytes(bytes),
        }
    }
}

// See OpenJDK: src/java.base/share/native/libjimage/imageFile.hpp.
struct Header {
    endian: Endian,
    major_version: u16,
    minor_version: u16,
    flags: u32,
    resource_count: u32,
    table_length: u32,
    locations_size: u32,
    strings_size: u32,
}

impl Header {
    fn parse(mut reader: impl Read) -> Result<Self, String> {
        // The magic identifies both the format and the image's byte order.
        let mut magic = [0; 4];
        reader
            .read_exact(&mut magic)
            .map_err(|error| error.to_string())?;
        let endian = match magic {
            [0xDA, 0xDA, 0xFE, 0xCA] => Endian::Little,
            [0xCA, 0xFE, 0xDA, 0xDA] => Endian::Big,
            _ => return Err("invalid jimage magic".into()),
        };

        let version = endian.read_u32(&mut reader)?;
        let header = Self {
            endian,
            major_version: (version >> 16) as u16,
            minor_version: version as u16,
            flags: endian.read_u32(&mut reader)?,
            resource_count: endian.read_u32(&mut reader)?,
            table_length: endian.read_u32(&mut reader)?,
            locations_size: endian.read_u32(&mut reader)?,
            strings_size: endian.read_u32(&mut reader)?,
        };

        if header.major_version != 1 || header.minor_version > 1 {
            return Err(format!(
                "unsupported jimage version: {}.{}",
                header.major_version, header.minor_version
            ));
        }

        Ok(header)
    }
}

fn string_bytes(strings: &[u8], offset: u64) -> Result<&[u8], String> {
    let offset = usize::try_from(offset).map_err(|error| error.to_string())?;
    let bytes = strings
        .get(offset..)
        .ok_or("jimage string offset out of bounds")?;
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .ok_or("unterminated jimage string")?;
    Ok(&bytes[..end])
}

fn string(strings: &[u8], offset: u64) -> Result<Cow<'_, str>, String> {
    cesu8::from_java_cesu8(string_bytes(strings, offset)?).map_err(|error| error.to_string())
}

#[derive(Debug)]
struct ImageLocation {
    module: u64,
    parent: u64,
    base: u64,
    extension: u64,
    resource_offset: u64,
    compressed_size: u64,
    uncompressed_size: u64,
    preview_flags: u64,
}

impl ImageLocation {
    fn class(
        &self,
        id: &Location,
        reader: &mut (impl Read + Seek),
        strings: &[u8],
        resources_start: u64,
        file_size: u64,
    ) -> Result<Option<(Location, Class)>, String> {
        if self.compressed_size != 0 {
            return Err("compressed jimage resources are unsupported".into());
        }

        // Compare ASCII bytes before decoding names or touching resource data.
        if string_bytes(strings, self.extension)? != b"class" {
            return Ok(None);
        }
        let module = string(strings, self.module)?;
        if module.is_empty() || module == "modules" || module == "packages" {
            return Ok(None);
        }

        let parent = string(strings, self.parent)?;
        let base = string(strings, self.base)?;
        let mut name = format!("/{module}/");
        if !parent.is_empty() {
            name.push_str(&parent);
            name.push('/');
        }
        name.push_str(&base);
        name.push_str(".class");
        let entry_id = id.entry(LocationEntry::Container(name));

        // Location offsets are relative to the resource section, not the file.
        let start = resources_start
            .checked_add(self.resource_offset)
            .ok_or_else(|| format!("{entry_id:?}: resource offset overflow"))?;
        let end = start
            .checked_add(self.uncompressed_size)
            .ok_or_else(|| format!("{entry_id:?}: resource size overflow"))?;
        if end > file_size {
            return Err(format!("{entry_id:?}: resource extends beyond jimage"));
        }

        // The class parser reads to EOF; limit it to this resource's range.
        reader
            .seek(SeekFrom::Start(start))
            .map_err(|error| format!("{entry_id:?}: {error}"))?;
        let mut resource = reader.take(self.uncompressed_size);
        let class = crate::parser::parse(&entry_id, &mut resource)
            .map_err(|error| format!("{entry_id:?}: {error}"))?;
        if resource.limit() != 0 {
            return Err(format!("{entry_id:?}: truncated jimage resource"));
        }
        Ok(class.map(|class| (entry_id, class)))
    }

    fn parse(mut reader: impl Read) -> Result<Self, String> {
        // Omitted attributes default to zero. See OpenJDK imageFile.hpp, Location.
        let mut attributes = [0; 9];
        loop {
            let mut tag = [0];
            reader
                .read_exact(&mut tag)
                .map_err(|error| error.to_string())?;

            // Upper five bits identify the attribute; kind zero ends the record.
            let kind = usize::from(tag[0] >> 3);
            if kind == 0 {
                break;
            }

            let attribute = attributes
                .get_mut(kind)
                .ok_or_else(|| format!("invalid jimage location attribute: {kind}"))?;

            // Lower three bits encode the value's byte length minus one.
            let length = usize::from(tag[0] & 7) + 1;
            let mut bytes = [0; 8];
            reader
                .read_exact(&mut bytes[8 - length..])
                .map_err(|error| error.to_string())?;

            // Location values are big-endian regardless of the image's byte order.
            *attribute = u64::from_be_bytes(bytes);
        }

        Ok(Self {
            module: attributes[1],
            parent: attributes[2],
            base: attributes[3],
            extension: attributes[4],
            resource_offset: attributes[5],
            compressed_size: attributes[6],
            uncompressed_size: attributes[7],
            preview_flags: attributes[8],
        })
    }
}

pub fn process(
    id: &Location,
    mut reader: impl Read + Seek,
) -> Result<Option<impl Iterator<Item = Result<(Location, Class), String>>>, String> {
    if !matches!(
        super::magic(&mut reader).map_err(|error| format!("{id:?}: {error}"))?,
        Some([0xCA, 0xFE, 0xDA, 0xDA] | [0xDA, 0xDA, 0xFE, 0xCA])
    ) {
        return Ok(None);
    }

    let header = Header::parse(&mut reader).map_err(|error| format!("{id:?}: {error}"))?;

    // Redirects serve name lookup; enumeration only needs the location offsets.
    let table_size = u64::from(header.table_length) * 4;
    let index_start = reader
        .seek(SeekFrom::Current(table_size as i64))
        .map_err(|error| format!("{id:?}: {error}"))?;

    // The remaining index is [location offsets | location records | strings].
    let index_size = table_size + u64::from(header.locations_size) + u64::from(header.strings_size);
    let resources_start = index_start
        .checked_add(index_size)
        .ok_or_else(|| format!("{id:?}: jimage index size overflow"))?;

    // Validate the index extent before allocating; retain the length for resource checks.
    let file_size = reader
        .seek(SeekFrom::End(0))
        .map_err(|error| format!("{id:?}: {error}"))?;
    if resources_start > file_size {
        return Err(format!("{id:?}: truncated jimage index"));
    }

    // Keep encoded metadata in one buffer; decode only as the iterator advances.
    let index_size = usize::try_from(index_size).map_err(|error| format!("{id:?}: {error}"))?;
    let mut index = vec![0; index_size];
    reader
        .seek(SeekFrom::Start(index_start))
        .map_err(|error| format!("{id:?}: {error}"))?;
    reader
        .read_exact(&mut index)
        .map_err(|error| format!("{id:?}: {error}"))?;

    // These boundaries are relative to the buffer, unlike resources_start.
    let locations_start = table_size as usize;
    let strings_start = locations_start + header.locations_size as usize;

    // Each four-byte slot points into location records; zero marks an unused slot.
    Ok(Some((0..locations_start).step_by(4).filter_map(
        move |slot| {
            let offset = header.endian.decode([
                index[slot],
                index[slot + 1],
                index[slot + 2],
                index[slot + 3],
            ]);
            if offset == 0 {
                return None;
            }

            let result = (|| {
                let bytes = index[locations_start..strings_start]
                    .get(offset as usize..)
                    .ok_or("jimage location offset out of bounds")?;
                let location = ImageLocation::parse(bytes)?;
                location.class(
                    id,
                    &mut reader,
                    &index[strings_start..],
                    resources_start,
                    file_size,
                )
            })();

            result
                .map_err(|error| format!("{id:?}: location {offset}: {error}"))
                .transpose()
        },
    )))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn class_bytes() -> Vec<u8> {
        let name = b"java/lang/Object";
        let mut bytes = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 52, 0, 3, 1];
        bytes.extend((name.len() as u16).to_be_bytes());
        bytes.extend(name);
        bytes.extend([7, 0, 1]);
        for value in [0x21u16, 2, 0, 0, 0, 0, 0] {
            bytes.extend(value.to_be_bytes());
        }
        bytes
    }

    fn image_bytes(
        encode: fn(u32) -> [u8; 4],
        compressed_size: u64,
        resource_offset: u64,
        resource_size: u64,
    ) -> Vec<u8> {
        let mut locations = vec![0];
        for (index, value) in [
            1,
            11,
            21,
            28,
            resource_offset,
            compressed_size,
            resource_size,
            0,
        ]
        .into_iter()
        .enumerate()
        {
            if value == 0 {
                continue;
            }
            let bytes = value.to_be_bytes();
            let start = bytes.iter().position(|byte| *byte != 0).unwrap();
            locations.push(((index as u8 + 1) << 3) | (7 - start as u8));
            locations.extend(&bytes[start..]);
        }
        locations.push(0);
        let strings = b"\0java.base\0java/lang\0Object\0class\0";
        let mut bytes = Vec::new();
        for field in [
            0xCAFEDADA,
            0x00010000,
            0,
            1,
            2,
            locations.len() as u32,
            strings.len() as u32,
        ] {
            bytes.extend(encode(field));
        }
        for field in [0, 0, 0, 1] {
            bytes.extend(encode(field));
        }
        bytes.extend(locations);
        bytes.extend(strings);
        bytes.extend(class_bytes());
        bytes
    }

    fn header_bytes(encode: fn(u32) -> [u8; 4], version: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [0xCAFEDADA, version, 0, 1, 1, 13, 34] {
            bytes.extend(encode(value));
        }
        bytes
    }

    fn location_bytes() -> Vec<u8> {
        vec![0x08, 1, 0x10, 11, 0x18, 21, 0x20, 28, 0x39, 1, 0x20, 0]
    }

    #[test]
    fn reads_supported_versions_in_both_byte_orders() {
        for (encode, endian) in [
            (u32::to_le_bytes as fn(u32) -> [u8; 4], Endian::Little),
            (u32::to_be_bytes, Endian::Big),
        ] {
            for version in [0x00010000, 0x00010001] {
                let mut bytes = header_bytes(encode, version);
                bytes.push(0xAA);
                let mut reader = bytes.as_slice();
                let header = Header::parse(&mut reader).unwrap();
                assert_eq!(header.endian, endian);
                assert_eq!(header.major_version, 1);
                assert_eq!(header.minor_version, version as u16);
                assert_eq!(header.flags, 0);
                assert_eq!(header.resource_count, 1);
                assert_eq!(header.table_length, 1);
                assert_eq!(header.locations_size, 13);
                assert_eq!(header.strings_size, 34);
                assert_eq!(reader, [0xAA]);
            }
        }
    }

    #[test]
    fn rejects_bad_magic_immediately() {
        let mut reader = [0, 0, 0, 0, 0xAA].as_slice();
        let error = Header::parse(&mut reader).err().unwrap();
        assert!(error.contains("invalid jimage magic"));
        assert_eq!(reader, [0xAA]);
    }

    #[test]
    fn rejects_unknown_versions_after_reading_the_header() {
        for encode in [u32::to_le_bytes, u32::to_be_bytes] {
            for version in [0, 0x00010002, 0x00020000] {
                let mut bytes = header_bytes(encode, version);
                bytes.push(0xAA);
                let mut reader = bytes.as_slice();
                let error = Header::parse(&mut reader).err().unwrap();
                assert!(error.contains("unsupported jimage version"));
                assert_eq!(reader, [0xAA]);
            }
        }
    }

    #[test]
    fn recognized_images_never_fall_back() {
        for encode in [u32::to_le_bytes, u32::to_be_bytes] {
            let truncated = encode(0xCAFEDADA);
            assert!(process(&Location::file("modules"), Cursor::new(truncated)).is_err());
            let unsupported = header_bytes(encode, 0x00020000);
            assert!(process(&Location::file("modules"), Cursor::new(unsupported)).is_err());
        }
    }

    #[test]
    fn reads_classes_from_mise_jdk_21() {
        let home = beans_testing::support::jdks::list("temurin-21.")
            .unwrap()
            .pop()
            .expect("Temurin JDK 21 is required; run mise install");
        let image = home.join("lib/modules");
        let file = std::fs::File::open(&image).unwrap();
        let id = Location::file(image);
        let entries = process(&id, file).unwrap().unwrap();
        let mut count = 0;
        let mut found_object = false;
        for entry in entries {
            let (entry_id, class) = entry.unwrap();
            count += 1;
            if class.name == "java/lang/Object" {
                assert_eq!(
                    entry_id,
                    id.entry(LocationEntry::Container(
                        "/java.base/java/lang/Object.class".into()
                    ))
                );
                found_object = true;
            }
        }
        assert!(count > 0);
        assert!(found_object);
    }

    #[test]
    fn reads_the_example_location() {
        let location = ImageLocation::parse(location_bytes().as_slice()).unwrap();
        assert_eq!(location.module, 1);
        assert_eq!(location.parent, 11);
        assert_eq!(location.base, 21);
        assert_eq!(location.extension, 28);
        assert_eq!(location.resource_offset, 0);
        assert_eq!(location.compressed_size, 0);
        assert_eq!(location.uncompressed_size, 288);
        assert_eq!(location.preview_flags, 0);
    }

    #[test]
    fn reads_eight_byte_values_and_preview_flags() {
        let bytes = [0x2F, 1, 2, 3, 4, 5, 6, 7, 8, 0x40, 3, 0];
        let location = ImageLocation::parse(bytes.as_slice()).unwrap();
        assert_eq!(location.resource_offset, 0x0102030405060708);
        assert_eq!(location.preview_flags, 3);
    }

    #[test]
    fn rejects_invalid_and_truncated_locations() {
        assert!(ImageLocation::parse([0x48, 1, 0].as_slice()).is_err());
        let bytes = location_bytes();
        for length in 0..bytes.len() {
            assert!(ImageLocation::parse(&bytes[..length]).is_err());
        }
    }

    #[test]
    fn rejects_invalid_location_offsets() {
        for encode in [u32::to_le_bytes, u32::to_be_bytes] {
            let mut bytes = image_bytes(encode, 0, 0, class_bytes().len() as u64);
            let locations_size = Header::parse(bytes.as_slice()).unwrap().locations_size;
            for offset in [locations_size, locations_size + 1, u32::MAX] {
                bytes[40..44].copy_from_slice(&encode(offset));
                let id = Location::file("modules");
                let mut entries = process(&id, Cursor::new(&bytes)).unwrap().unwrap();
                assert!(entries.next().unwrap().is_err());
            }
        }
    }

    #[test]
    fn rejects_compressed_resources() {
        for encode in [u32::to_le_bytes, u32::to_be_bytes] {
            let bytes = image_bytes(encode, 42, 0, class_bytes().len() as u64);
            let id = Location::file("modules");
            let mut entries = process(&id, Cursor::new(bytes)).unwrap().unwrap();
            let error = entries.next().unwrap().err().unwrap();
            assert!(error.contains("location 1: compressed jimage resources are unsupported"));
        }
    }

    #[test]
    fn yields_classes_in_both_byte_orders() {
        for encode in [u32::to_le_bytes, u32::to_be_bytes] {
            let mut bytes = image_bytes(encode, 0, 0, class_bytes().len() as u64);
            bytes.extend([0xAA; 16]);
            let id =
                Location::file("outer.zip").entry(LocationEntry::Container("lib/modules".into()));
            let mut entries = process(&id, Cursor::new(bytes)).unwrap().unwrap();
            let (entry_id, class) = entries.next().unwrap().unwrap();
            assert_eq!(class.name, "java/lang/Object");
            assert_eq!(
                entry_id,
                id.entry(LocationEntry::Container(
                    "/java.base/java/lang/Object.class".into()
                ))
            );
            assert!(entries.next().is_none());
        }
    }

    #[test]
    fn rejects_truncated_images() {
        let bytes = image_bytes(u32::to_le_bytes, 0, 0, class_bytes().len() as u64);
        for length in 0..bytes.len() {
            match process(&Location::file("modules"), Cursor::new(&bytes[..length])) {
                Err(_) => {}
                Ok(None) => assert!(length < 4),
                Ok(Some(mut entries)) => assert!(entries.next().unwrap().is_err()),
            }
        }
    }

    #[test]
    fn rejects_resource_range_overflow() {
        for (offset, size) in [(u64::MAX, 1), (0, u64::MAX)] {
            let bytes = image_bytes(u32::to_le_bytes, 0, offset, size);
            let id = Location::file("modules");
            let mut entries = process(&id, Cursor::new(bytes)).unwrap().unwrap();
            assert!(entries.next().unwrap().err().unwrap().contains("overflow"));
        }
    }

    #[test]
    fn reads_modified_utf8_strings() {
        let bytes = b"\0Object\0\xC0\x80\0\xED\xA0\xBD\xED\xB8\x80\0";
        assert_eq!(string(bytes, 0).unwrap(), "");
        assert_eq!(string(bytes, 1).unwrap(), "Object");
        assert_eq!(string(bytes, 8).unwrap(), "\0");
        assert_eq!(string(bytes, 11).unwrap(), "😀");
        assert!(string(bytes, bytes.len() as u64).is_err());
        assert!(string(bytes, u64::MAX).is_err());
        for bytes in [b"unterminated".as_slice(), b"\xFF\0"] {
            assert!(string(bytes, 0).is_err());
        }
    }

    #[test]
    fn skips_non_class_resources() {
        let strings = b"\0java.base\0java/lang\0Object\0txt\0";
        let mut location = ImageLocation::parse(location_bytes().as_slice()).unwrap();
        location.module = u64::MAX;
        let result = location
            .class(
                &Location::file("modules"),
                &mut Cursor::new([]),
                strings,
                0,
                0,
            )
            .unwrap();
        assert!(result.is_none());
    }
}
