use crate::error::{FontError, Tag};
use crate::parse::Parser;
use crate::write::Writer;
use std::io::{Read, Write};

#[derive(Debug, Clone, PartialEq)]
pub struct Woff2Table {
    pub tag: Tag,
    pub orig_length: u32,
    pub transform_length: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Woff2Header {
    pub signature: u32,
    pub flavor: u32,
    pub length: u32,
    pub num_tables: u16,
    pub reserved: u16,
    pub total_sfnt_size: u32,
    pub total_compressed_size: u32,
    pub major_version: u16,
    pub minor_version: u16,
    pub meta_offset: u32,
    pub meta_length: u32,
    pub meta_orig_length: u32,
    pub priv_offset: u32,
    pub priv_length: u32,
}

pub fn read_woff2(buf: &[u8]) -> Result<Vec<(Tag, Vec<u8>)>, FontError> {
    let mut p = Parser::new(buf, 0);
    let signature = p.u32()?;
    if signature != 0x774F4632 {
        return Err(FontError::invalid_table(
            Tag::new(b"wOF2"),
            &format!("Expected WOFF2 signature 0x774F4632, got 0x{:08X}", signature),
        ));
    }
    let _flavor = p.u32()?;
    let _length = p.u32()?;
    let num_tables = p.u16()?;
    let _reserved = p.u16()?;
    let _total_sfnt_size = p.u32()?;
    let total_compressed_size = p.u32()?;
    let _major_version = p.u16()?;
    let _minor_version = p.u16()?;
    let _meta_offset = p.u32()?;
    let _meta_length = p.u32()?;
    let _meta_orig_length = p.u32()?;
    let _priv_offset = p.u32()?;
    let _priv_length = p.u32()?;

    let header_size = 48;
    let compressed_data = &buf[header_size..header_size + total_compressed_size as usize];

    let mut decompressed = Vec::new();
    {
        let mut decoder = brotli::Decompressor::new(compressed_data, 4096);
        decoder.read_to_end(&mut decompressed).map_err(|e| {
            FontError::invalid_table(Tag::new(b"wOF2"), &format!("Brotli decompression failed: {}", e))
        })?;
    }

    let mut dp = Parser::new(&decompressed, 0);
    let mut tables = Vec::with_capacity(num_tables as usize);

    for _ in 0..num_tables {
        let flags = dp.u8()?;
        let table_type = flags & 0x3F;
        let has_transform = (flags >> 6) & 0x01 != 0;

        let tag = if table_type == 0x3F {
            let t = dp.tag()?;
            t
        } else {
            let known = [
                b"cmap", b"head", b"hhea", b"hmtx", b"maxp", b"name", b"OS/2",
                b"post", b"cvt ", b"fpgm", b"glyf", b"loca", b"prep", b"CFF ",
                b"CFF2", b"GPOS", b"GSUB", b"HVAR", b"JSTF", b"MVAR", b"BASE",
                b"GDEF", b"VDMX", b"vhea", b"vmtx", b"STAT", b"avar", b"cvar",
                b"fvar", b"gvar", b"HVAR", b"MVAR", b"DSIG", b"EBDT", b"EBLC",
                b"EBSC", b"gasp", b"hdmx", b"kern", b"LTSH", b"MERG", b"meta",
                b"PCLT", b"VDMX", b"vhea", b"vmtx", b"MATH", b"CPAL", b"SVG ",
                b"sbix", b"CBDT", b"CBLC", b"COLR", b"JSTF", b"DSIG", b"EBDT",
            ];
            if (table_type as usize) < known.len() {
                Tag::new(known[table_type as usize])
            } else {
                Tag::new(b"unkn")
            }
        };

        let orig_length = read_woff2_uint(&mut dp)?;
        let transform_length = if has_transform {
            read_woff2_uint(&mut dp)?
        } else {
            orig_length
        };

        tables.push(Woff2Table {
            tag,
            orig_length,
            transform_length,
            data: vec![],
        });
    }

    // After directory, the actual table data follows in the decompressed stream
    let data_start = dp.offset();
    let mut table_data = Vec::with_capacity(tables.len());
    let mut offset = data_start;
    for t in &tables {
        let len = t.transform_length as usize;
        if offset + len > decompressed.len() {
            return Err(FontError::invalid_table(
                Tag::new(b"wOF2"),
                "Table data exceeds decompressed stream",
            ));
        }
        table_data.push(decompressed[offset..offset + len].to_vec());
        offset += len;
    }

    // Reconstruct sfnt table list
    let mut result = Vec::with_capacity(tables.len());
    for (i, t) in tables.iter().enumerate() {
        result.push((t.tag.clone(), table_data[i].clone()));
    }
    Ok(result)
}

pub fn write_woff2(tables: &[(Tag, Vec<u8>)]) -> Result<Vec<u8>, FontError> {
    // Build sfnt-style concatenated table data for Brotli compression
    let mut sfnt_data = Writer::new();
    let num_tables = tables.len() as u16;
    let mut table_entries: Vec<(Tag, u32, u32, Vec<u8>)> = Vec::with_capacity(tables.len());

    for (tag, data) in tables {
        let orig_len = data.len() as u32;
        let mut padded = data.clone();
        while padded.len() % 4 != 0 {
            padded.push(0);
        }
        let start = sfnt_data.len() as u32;
        sfnt_data.write_bytes(&padded);
        table_entries.push((*tag, start, orig_len, data.clone()));
    }

    // Build WOFF2 table directory
    let mut dir = Writer::new();
    for (tag, _offset, orig_len, _data) in &table_entries {
        // Find known tag index
        let known = [
            b"cmap", b"head", b"hhea", b"hmtx", b"maxp", b"name", b"OS/2",
            b"post", b"cvt ", b"fpgm", b"glyf", b"loca", b"prep", b"CFF ",
            b"CFF2", b"GPOS", b"GSUB", b"HVAR", b"JSUF", b"MVAR", b"BASE",
            b"GDEF", b"VDMX", b"vhea", b"vmtx", b"STAT", b"avar", b"cvar",
            b"fvar", b"gvar", b"HVAR", b"MVAR", b"DSIG", b"EBDT", b"EBLC",
            b"EBSC", b"gasp", b"hdmx", b"kern", b"LTSH", b"MERG", b"meta",
            b"PCLT", b"VDMX", b"vhea", b"vmtx", b"MATH", b"CPAL", b"SVG ",
            b"sbix", b"CBDT", b"CBLC", b"COLR", b"JSUF", b"DSIG", b"EBDT",
        ];
        let flags = if let Some(idx) = known.iter().position(|&k| *k == tag.0) {
            idx as u8
        } else {
            0x3F
        };
        dir.write_u8(flags);
        if flags == 0x3F {
            dir.write_tag(&tag.0);
        }
        write_woff2_uint(&mut dir, *orig_len);
        // Only write transformLength if transform version > 0
        // Basic implementation does not use transforms
        // write_woff2_uint(&mut dir, data.len() as u32);
    }

    // Concatenate directory + table data
    let mut uncompressed = dir.into_vec();
    for (_, _, _, data) in &table_entries {
        uncompressed.extend_from_slice(data);
    }

    // Brotli compress
    let mut compressed = Vec::new();
    {
        let mut encoder = brotli::CompressorWriter::new(&mut compressed, 4096, 4, 22);
        encoder.write_all(&uncompressed).map_err(|e| {
            FontError::invalid_table(Tag::new(b"wOF2"), &format!("Brotli compression failed: {}", e))
        })?;
    }

    let total_sfnt_size = 12 + tables.len() * 16 + sfnt_data.len();

    let mut w = Writer::new();
    w.write_u32(0x774F4632); // signature
    w.write_u32(0x00010000); // flavor (TrueType)
    let _length_placeholder = w.len();
    w.write_u32(0); // length placeholder
    w.write_u16(num_tables);
    w.write_u16(0); // reserved
    w.write_u32(total_sfnt_size as u32);
    w.write_u32(compressed.len() as u32);
    w.write_u16(1); // majorVersion
    w.write_u16(0); // minorVersion
    w.write_u32(0); // metaOffset
    w.write_u32(0); // metaLength
    w.write_u32(0); // metaOrigLength
    w.write_u32(0); // privOffset
    w.write_u32(0); // privLength
    w.write_bytes(&compressed);

    let length = w.len() as u32;
    let bytes = w.into_vec();
    // Patch length
    let mut patched = bytes.clone();
    patched[8..12].copy_from_slice(&length.to_be_bytes());

    Ok(patched)
}

fn read_woff2_uint(p: &mut Parser) -> Result<u32, FontError> {
    let b0 = p.u8()?;
    if b0 < 0x80 {
        Ok(b0 as u32)
    } else if b0 < 0xC0 {
        let b1 = p.u8()?;
        Ok(((b0 & 0x7F) as u32) << 8 | (b1 as u32))
    } else {
        let b1 = p.u8()?;
        let b2 = p.u8()?;
        let b3 = p.u8()?;
        Ok(((b0 & 0x3F) as u32) << 24 | (b1 as u32) << 16 | (b2 as u32) << 8 | (b3 as u32))
    }
}

fn write_woff2_uint(w: &mut Writer, val: u32) {
    if val < 128 {
        w.write_u8(val as u8);
    } else if val < 0x4000 {
        w.write_u8(((val >> 8) as u8) | 0x80);
        w.write_u8((val & 0xFF) as u8);
    } else {
        w.write_u8(((val >> 24) as u8) | 0xC0);
        w.write_u8(((val >> 16) & 0xFF) as u8);
        w.write_u8(((val >> 8) & 0xFF) as u8);
        w.write_u8((val & 0xFF) as u8);
    }
}
