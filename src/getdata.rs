use std::{
    io::{Read, Seek},
    ops::{Add, Mul},
    path,
};

use num::cast::AsPrimitive;
impl crate::Dirfile {
    pub fn getraw<T>(
        &self,
        entry_raw: &crate::EntryRaw,
        path: std::path::PathBuf,
        endian: crate::Endian,
        first_frame: usize,
        first_sample: usize,
        num_frames: usize,
        num_samples: usize,
    ) -> Vec<T>
    where
        T: 'static + Copy,
        f64: AsPrimitive<T>,
        u32: AsPrimitive<T>,
        u64: AsPrimitive<T>,
    {
        fn read_data<T, F>(
            reader: &mut (impl Read + Seek),
            offset: usize,
            length: usize,
            buf_size: usize,
            convert: F,
        ) -> Vec<T>
        where
            F: Fn(&[u8]) -> T,
        {
            let mut data: Vec<T> = Vec::with_capacity(length / buf_size);
            let mut buf = vec![0; buf_size];
            reader
                .seek(std::io::SeekFrom::Start((offset * buf_size) as u64))
                .unwrap();
            while let Ok(()) = reader.read_exact(&mut buf) {
                let value = convert(&buf);
                data.push(value);
                if data.capacity() == 0 {
                    break;
                }
            }
            data
        }

        let offset = first_sample + first_frame * entry_raw.spf as usize;
        let length = num_frames * entry_raw.spf as usize + num_samples;
        let file = std::fs::File::open(path).unwrap();

        let mut reader = std::io::BufReader::new(file);
        match entry_raw.data_type {
            crate::RawTypes::Float64 => {
                read_data(&mut reader, offset, length, 8, |buf| match endian {
                    crate::Endian::Big => f64::from_be_bytes(buf.try_into().unwrap()).as_(),
                    crate::Endian::Little => f64::from_le_bytes(buf.try_into().unwrap()).as_(),
                })
            }
            crate::RawTypes::Uint64 => {
                read_data(&mut reader, offset, length, 8, |buf| match endian {
                    crate::Endian::Big => u64::from_be_bytes(buf.try_into().unwrap()).as_(),
                    crate::Endian::Little => u64::from_le_bytes(buf.try_into().unwrap()).as_(),
                })
            }
            crate::RawTypes::Uint32 => {
                read_data(&mut reader, offset, length, 4, |buf| match endian {
                    crate::Endian::Big => u32::from_be_bytes(buf.try_into().unwrap()).as_(),
                    crate::Endian::Little => u32::from_le_bytes(buf.try_into().unwrap()).as_(),
                })
            } // _ => {
              //     unimplemented!("data type {:?}", entry_raw.data_type)
              // }
        }
    }

    pub fn getbit<T>(
        &self,
        entry_bit: &crate::EntryBit,
        first_frame: usize,
        first_sample: usize,
        num_frames: usize,
        num_samples: usize,
    ) -> Vec<T>
    where
        T: 'static + Copy,
        f64: AsPrimitive<T>,
        u32: AsPrimitive<T>,
        u64: AsPrimitive<T>,
    {
        //get underlying data
        let inner = self.getdata::<u64>(
            &entry_bit.parent_field,
            first_frame,
            first_sample,
            num_frames,
            num_samples,
        );
        //create a bit mask using entry_bit.num_bits and .first_bit
        //then apply this bitmask to resut
        let mask = u64::max_value() >> (u64::BITS - entry_bit.num_bits) << entry_bit.start_bit;
        let data: Vec<T> = inner.into_iter().map(|val| (val & mask).as_()).collect();
        return data;
    }
    pub fn getlincom<T>(
        &self,
        entry_lincom: &crate::EntryLincom,
        first_frame: usize,
        first_sample: usize,
        num_frames: usize,
        num_samples: usize,
    ) -> Vec<T>
    where
        T: 'static + Copy + std::ops::Mul<Output = T> + Add<Output = T> + AsPrimitive<T>,
        f64: AsPrimitive<T>,
        u32: AsPrimitive<T>,
        i32: AsPrimitive<T>,
        u64: AsPrimitive<T>,
    {
        let mut entry_lincom = entry_lincom;
        let mut data: Vec<T> = Vec::new();
        loop {
            //get underlying data
            let inner = self.getdata::<T>(
                &entry_lincom.parent_field,
                first_frame,
                first_sample,
                num_frames,
                num_samples,
            );
            if data.len() == 0 {
                data = vec![0.as_(); inner.len()]
            }
            data = data
                .into_iter()
                .zip(inner.into_iter())
                .map(|(d, val)| d + entry_lincom.m.as_() * val + entry_lincom.b.as_())
                .collect();
            if entry_lincom.next_term.is_none() {
                return data;
            }
            entry_lincom = entry_lincom
                .next_term
                .as_ref()
                .expect("just checked its there");
        }
    }
    pub fn getlinterp<T>(
        &self,
        entry_linterp: &crate::EntryLinterp,
        first_frame: usize,
        first_sample: usize,
        num_frames: usize,
        num_samples: usize,
    ) -> Vec<T>
    where
    T: 'static + Copy + std::ops::Mul<Output = T> + Add<Output = T> + AsPrimitive<T>,
        f64: AsPrimitive<T>,
        u32: AsPrimitive<T>,
        i32: AsPrimitive<T>,
        u64: AsPrimitive<T>,
    {
        let linterp = |val: f64| -> f64 {
            // do binary search to find val inside entry_linterp.x
            // then use the index to find the corresponding value in entry_linterp.y
            let x = &entry_linterp.x;
            let y = &entry_linterp.y;
            let mut low = 0;
            let mut high = x.len();
            while low < high {
                let mid = (low + high) / 2;
                if x[mid] < val {
                    low = mid + 1;
                } else {
                    high = mid;
                }
            }
            // linear interpolation between points
            if low == 0 {
                y[0]
            } else if low == x.len() {
                y[y.len() - 1]
            } else {
                let x1 = x[low - 1];
                let x2 = x[low];
                let y1 = y[low - 1];
                let y2 = y[low];
                y1 + (val - x1) * (y2 - y1) / (x2 - x1)
            }
        };

        let inner = self.getdata::<f64>(
            &entry_linterp.parent_field,
            first_frame,
            first_sample,
            num_frames,
            num_samples,
        );

        let result: Vec<T> = inner.into_iter().map(|val| linterp(val).as_()).collect();
        return result;
    }
}
