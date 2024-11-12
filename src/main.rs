use num::cast::AsPrimitive;
use std::{
    any::TypeId,
    collections::HashMap,
    error::Error,
    io::{BufRead, Read, Seek, Write},
    ops::Add,
};
type Result<T> = std::result::Result<T, Box<dyn Error>>;

mod format;
mod getdata;
mod putdata;

#[derive(Debug)]
struct Entry {
    entry_type: EntryType,
    name: String,
    dirfile_options: DirfileOptions,
    dirfile_path: std::path::PathBuf,
}

#[derive(Debug)]
struct EntryRaw {
    spf: u32,
    data_type: RawTypes,
}
#[derive(Debug)]
enum RawTypes {
    Uint32,
    Uint64,
    Float64,
}

impl From<&str> for RawTypes {
    fn from(value: &str) -> Self {
        match value {
            "UINT32" => RawTypes::Uint32,
            "UINT64" => RawTypes::Uint64,
            "FLOAT64" => RawTypes::Float64,
            _ => panic!("Unknown raw type {}", value),
        }
    }
}

#[derive(Debug)]
struct EntryBit {
    start_bit: u32,
    num_bits: u32,
    parent_field: String,
}

#[derive(Debug)]
struct EntryLincom {
    parent_field: String,
    m: f64,                              //should be complex
    b: f64,                              //should be complex
    next_term: Option<Box<EntryLincom>>, //recurse
}

#[derive(Debug)]
struct EntryLinterp {
    parent_field: String,
    lookup_table_path: std::path::PathBuf,
    x: Vec<f64>, //should be complex
    y: Vec<f64>, //should be complex
}

#[derive(Debug)]
enum EntryType {
    Raw(EntryRaw),
    Bit(EntryBit),
    Lincom(EntryLincom),
    Linterp(EntryLinterp),
}

struct Dirfile {
    entries: HashMap<String, Entry>,
    root_dir: std::path::PathBuf,
    fragments: HashMap<String, Dirfile>,
}

#[derive(Debug, Clone, Copy)]
struct DirfileOptions {
    pub version: u32,
    pub endian: Endian,
    pub encoding: Option<Encoding>,
}
#[derive(Debug, Clone, Copy)]
enum Endian {
    Big,
    Little,
}

impl From<&str> for Endian {
    fn from(value: &str) -> Self {
        match value {
            "big" => Endian::Big,
            "little" => Endian::Little,
            _ => panic!("Unknown endian {}", value),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Encoding {
    Sie,
}

impl From<&format::FieldDefinition> for EntryType {
    fn from(value: &format::FieldDefinition) -> Self {
        match value.field_type.as_str() {
            "RAW" => {
                let data_type = RawTypes::from(value.args[0].as_str());
                let spf = value.args[1].parse().unwrap();
                EntryType::Raw(EntryRaw { spf, data_type })
            }
            "BIT" => {
                let start_bit = value.args[1].parse().unwrap();
                let num_bits = value.args[2].parse().unwrap();
                let parent_field = value.args[0].clone();
                EntryType::Bit(EntryBit {
                    start_bit,
                    num_bits,
                    parent_field,
                })
            }
            "LINCOM" => {
                let mut args = value.args.clone();
                if args.len() % 3 != 0 {
                    args = args[1..].to_vec();
                }

                fn make_lincom(args: Vec<String>) -> EntryLincom {
                    let parent_field = args[0].clone();
                    let m: f64 = args[1].parse().unwrap();
                    let b: f64 = args[2].parse().unwrap();
                    if args.len() > 3 {
                        let next_term = make_lincom(args[3..].to_vec());
                        EntryLincom {
                            parent_field,
                            m,
                            b,
                            next_term: Some(Box::new(next_term)),
                        }
                    } else {
                        EntryLincom {
                            parent_field,
                            m,
                            b,
                            next_term: None,
                        }
                    }
                }

                EntryType::Lincom(make_lincom(args))
            }
            _ => panic!("Unknown field type {:?}", value),
        }
    }
}

impl Dirfile {
    fn new(root_dir: std::path::PathBuf) -> Result<Dirfile> {
        //parse in the format file which should be in the root_dir
        let format_file = root_dir.join("format");
        let format_file = std::fs::read_to_string(format_file)?;
        //split it into
        // println!("original: {}", format_file);

        let (_, parsed) = format::parse_format_file(&format_file).unwrap();
        // println!("we parsed {:?}", parsed);
        let mut dirfile_options = DirfileOptions {
            version: 0,
            endian: Endian::Big,
            encoding: None,
        };
        let mut entries = HashMap::new();
        let mut fragments = HashMap::new();
        for line in parsed {
            match line {
                format::Line::Directive(directive, args) => {
                    println!("directive: {:?} with args {:?}", directive, args);
                    match directive {
                        format::Directive::Version => {
                            dirfile_options.version = args[0].parse().unwrap();
                        }
                        format::Directive::Endian => {
                            dirfile_options.endian = Endian::from(args[0].as_str());
                        }
                        format::Directive::Encoding => {
                            assert_eq!(args[0].as_str(), "none");
                        }
                        format::Directive::Alias => {
                            panic!("alias not implemented");
                        }
                        format::Directive::Protect => {
                            println!("Warning: protect not implemented");
                        }
                        format::Directive::Reference => {
                            println!("Warning: reference not implemented");
                        }
                        format::Directive::Include => {
                            if args.len() !=0 {
                                panic!("Does not support include with namespace or su/pre fix");
                            }
                            fragments.insert(
                                args[0].clone(),
                                Dirfile::new(root_dir.join(args[0].clone())).unwrap(),
                            );
                        }
                    }
                }
                format::Line::FieldDefinition(field_definition) => {
                    // println!("field_definition: {:?}", field_definition);
                    let entry_type = EntryType::from(&field_definition);
                    let entry = Entry {
                        entry_type,
                        name: field_definition.name.clone(),
                        dirfile_options: (dirfile_options.clone()),
                        dirfile_path: (root_dir.clone()),
                    };
                    println!("entry: {:?}", entry);
                    entries.insert(field_definition.name.clone(), entry);
                }
            }
        }

        return Ok(Dirfile {
            entries,
            root_dir,
            fragments,
        });
    }

}

fn main() {
    println!("Hello, world!");
    let root_dir = std::path::PathBuf::from("data_test");
    let dirfile = Dirfile::new(root_dir).unwrap();
    dirfile.putdata("test", 0, 0, vec![1; 100]);
    let res = dirfile.getdata::<i64>("test", 1, 0, 100, 0);
    println!("res: {:?}", res);
    dirfile.putdata("test", 0, 0, vec![2; 100]);
    let res = dirfile.getdata::<i64>("test", 1, 0, 100, 0);
    println!("res: {:?}", res);
    dirfile.putdata("testuint", 0, 0, vec![3; 100]);
    let res = dirfile.getdata::<i64>("testuint", 1, 0, 100, 0);
    println!("res: {:?}", res);
    let res = dirfile.getdata::<i64>("testbit", 1, 0, 100, 0);
    println!("res: {:?}", res);
    let res = dirfile.getdata::<f64>("testlincom", 1, 0, 100, 0);
    println!("res: {:?}", res);
}
