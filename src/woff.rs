use crate::error::{FontError, Tag};
use crate::parse::Parser;
use crate::write::Writer;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{Read, Write};

const WOFF_SIGNATURE: u32 = 0x774F4646; // 'wOFF'

#[derive(Debug, Clone)]
pub struct WoffTableRecord {
    pub tag: Tag,
    pub offset: u32,
    pub comp_length: u32,
    pub orig_length: u32,
    pub orig_checksum: u32,
}

/// Read a WOFF file and return the decompressed sfnt data.
pub fn read_woff(buf: &[u8]) -> Result<Vec<u8>, FontError> {
    let mut p = Parser::new(buf, 0);
    let signature = p.u32()?;
    if signature != WOFF_SIGNATURE {
        return Err(FontError::invalid_table(Tag::new(b"sfnt"), "not a WOFF file"));
    }
    let _flavor = p.u32()?;
    let _length = p.u32()?;
    let num_tables = p.u16()?;
    let _reserved = p.u16()?;
    let _total_sfnt_size = p.u32()?;
    let _major_version = p.u16()?;
    let _minor_version = p.u16()?;
    let _meta_offset = p.u32()?;
    let _meta_length = p.u32()?;
    let _meta_orig_length = p.u32()?;
    let _priv_offset = p.u32()?;
    let _priv_length = p.u32()?;

    let mut records = Vec::with_capacity(num_tables as usize);
    for _ in 0..num_tables {
        records.push(WoffTableRecord {
            tag: p.tag()?,
            offset: p.u32()?,
            comp_length: p.u32()?,
            orig_length: p.u32()?,
            orig_checksum: p.u32()?,
        });
    }

    // Calculate total sfnt size
    let header_size = 12 + 16 * num_tables as u32;
    let mut total_size = header_size;
    for rec in &records {
        total_size += rec.orig_length;
        // 4-byte padding
        while total_size % 4 != 0 {
            total_size += 1;
        }
    }

    let mut sfnt = Writer::new();
    // sfnt header
    sfnt.write_u32(0x00010000); // TrueType flavor
    sfnt.write_u16(num_tables);
    let search_range = 1u16 << (num_tables as u32).ilog2();
    sfnt.write_u16(search_range * 16);
    sfnt.write_u16((num_tables as u32).ilog2() as u16);
    sfnt.write_u16(num_tables * 16 - search_range * 16);

    // Calculate sfnt offsets
    let mut current_offset = header_size;
    let mut sfnt_records = Vec::new();
    for rec in &records {
        sfnt_records.push((rec.tag, current_offset, rec.orig_length, rec.orig_checksum));
        current_offset += rec.orig_length;
        while current_offset % 4 != 0 {
            current_offset += 1;
        }
    }

    // Write sfnt table directory
    for (tag, offset, _length, checksum) in &sfnt_records {
        sfnt.write_tag(&tag.0);
        sfnt.write_u32(*checksum);
        sfnt.write_u32(*offset);
        sfnt.write_u32(*_length);
    }

    // Write table data
    let sfnt_data = sfnt.into_vec();
    let mut result = Vec::with_capacity(total_size as usize);
    result.extend_from_slice(&sfnt_data);

    for rec in &records {
        let data = &buf[rec.offset as usize..(rec.offset + rec.comp_length) as usize];
        let decompressed = if rec.comp_length != rec.orig_length {
            let mut decoder = ZlibDecoder::new(data);
            let mut out = Vec::with_capacity(rec.orig_length as usize);
            decoder.read_to_end(&mut out).map_err(|e| FontError::Io(e))?;
            out
        } else {
            data.to_vec()
        };
        result.extend_from_slice(&decompressed);
        // Pad to 4-byte boundary
        while result.len() % 4 != 0 {
            result.push(0);
        }
    }

    Ok(result)
}

/// Write a WOFF file from sfnt table data.
pub fn write_woff(tables: &[(Tag, Vec<u8>)]) -> Result<Vec<u8>, FontError> {
    let num_tables = tables.len() as u16;
    let search_range = 1u16 << (num_tables as u32).ilog2();
    let _entry_selector = (num_tables as u32).ilog2() as u16;
    let _range_shift = num_tables * 16 - search_range * 16;

    // Compress each table
    let mut compressed: Vec<(Tag, Vec<u8>, u32, u32)> = Vec::with_capacity(tables.len());
    for (tag, data) in tables {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).map_err(|e| FontError::Io(e))?;
        let comp = encoder.finish().map_err(|e| FontError::Io(e))?;
        let orig_checksum = calc_checksum(data);
        compressed.push((*tag, comp, data.len() as u32, orig_checksum));
    }

    let woff_header_size = 44u32;
    let woff_dir_size = 20 * num_tables as u32;
    let mut table_offset = woff_header_size + woff_dir_size;

    let mut woff = Writer::new();
    woff.write_u32(WOFF_SIGNATURE);
    woff.write_u32(0x00010000); // flavor
    // total length placeholder
    let total_length_pos = woff.len();
    woff.write_u32(0);
    woff.write_u16(num_tables);
    woff.write_u16(0); // reserved
    // total sfnt size placeholder
    let sfnt_size_pos = woff.len();
    woff.write_u32(0);
    woff.write_u16(1); // major version
    woff.write_u16(0); // minor version
    woff.write_u32(0); // meta offset
    woff.write_u32(0); // meta length
    woff.write_u32(0); // meta orig length
    woff.write_u32(0); // priv offset
    woff.write_u32(0); // priv length

    let mut record_offsets = Vec::new();
    for (tag, comp, orig_len, checksum) in &compressed {
        woff.write_tag(&tag.0);
        record_offsets.push(table_offset);
        woff.write_u32(table_offset);
        woff.write_u32(comp.len() as u32);
        woff.write_u32(*orig_len);
        woff.write_u32(*checksum);
        table_offset += comp.len() as u32;
        while table_offset % 4 != 0 {
            table_offset += 1;
        }
    }

    // Write table data
    for (i, (_tag, comp, _, _)) in compressed.iter().enumerate() {
        let expected_offset = record_offsets[i] as usize;
        while woff.len() < expected_offset {
            woff.write_u8(0);
        }
        woff.write_bytes(comp);
        while woff.len() % 4 != 0 {
            woff.write_u8(0);
        }
    }

    let result = woff.into_vec();
    let mut woff_final = result.clone();
    // Patch total length
    let total_length = result.len() as u32;
    woff_final[total_length_pos..total_length_pos + 4].copy_from_slice(&total_length.to_be_bytes());

    // Calculate total sfnt size
    let header_size = 12 + 16 * num_tables as u32;
    let mut total_sfnt = header_size;
    for (_, _, orig_len, _) in &compressed {
        total_sfnt += orig_len;
        while total_sfnt % 4 != 0 {
            total_sfnt += 1;
        }
    }
    woff_final[sfnt_size_pos..sfnt_size_pos + 4].copy_from_slice(&total_sfnt.to_be_bytes());

    Ok(woff_final)
}

fn calc_checksum(data: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    let mut i = 0;
    let mut padded = data.to_vec();
    while padded.len() % 4 != 0 {
        padded.push(0);
    }
    while i + 4 <= padded.len() {
        sum = sum.wrapping_add(u32::from_be_bytes([
            padded[i], padded[i + 1], padded[i + 2], padded[i + 3],
        ]));
        i += 4;
    }
    sum
}
