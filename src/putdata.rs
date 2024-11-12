use std::io::{Seek, Write};

use num::cast::AsPrimitive;

impl crate::Dirfile {
    pub fn putdata<T>(&self, name: &str, first_frame: usize, first_sample: usize, data: Vec<T>)
    where
        T: 'static + Copy,
        T: AsPrimitive<f64>,
        T: AsPrimitive<f32>,
        T: AsPrimitive<u32>,
        T: AsPrimitive<u64>,
    {
        fn write_data<T, F>(
            writer: &mut (impl Write + Seek),
            data: Vec<T>,
            offset: usize,
            buf_size: usize,
            convert: F,
        ) where
            F: Fn(&mut [u8], T),
        {
            let mut buf = vec![0; buf_size];
            writer
                .seek(std::io::SeekFrom::Start((offset * buf_size) as u64))
                .unwrap();
            for value in data {
                convert(&mut buf, value);
                writer.write_all(&buf).unwrap();
            }
            writer.flush().unwrap();
        }
        let entry = self.entries.get(name).unwrap();
        let path = entry.dirfile_path.join(name);
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .unwrap();

        let mut writer = std::io::BufWriter::new(file);
        match &entry.entry_type {
            crate::EntryType::Raw(raw) => {
                let offset = raw.spf as usize * first_frame  + first_sample;
                match raw.data_type {
                    crate::RawTypes::Float64 => {
                        write_data(&mut writer, data, offset, 8, |buf, value| {
                            let value: f64 = value.as_();
                            match entry.dirfile_options.endian {
                                crate::Endian::Big => buf.copy_from_slice(&value.to_be_bytes()),
                                crate::Endian::Little => buf.copy_from_slice(&value.to_le_bytes()),
                            }
                        });
                    }
                    crate::RawTypes::Uint32 => {
                        write_data(&mut writer, data, offset, 4, |buf, value| {
                            let value: u32 = value.as_();
                            match entry.dirfile_options.endian {
                                crate::Endian::Big => buf.copy_from_slice(&value.to_be_bytes()),
                                crate::Endian::Little => buf.copy_from_slice(&value.to_le_bytes()),
                            }
                        });
                    }
                    crate::RawTypes::Uint64 => {
                        write_data(&mut writer, data, offset, 8, |buf, value| {
                            let value: u64 = value.as_();
                            match entry.dirfile_options.endian {
                                crate::Endian::Big => buf.copy_from_slice(&value.to_be_bytes()),
                                crate::Endian::Little => buf.copy_from_slice(&value.to_le_bytes()),
                            }
                        });
                    }
                }
                writer.flush().unwrap(); // Flush once after the loop
            }
            _ => {
                panic!("can only put raw data into a file - derived fields are read only")
            }
        }
    }
}
