pub mod addr;
pub mod block_set;
pub mod cfg;
pub mod cfg_func;
pub mod ins;
pub mod obj;
pub mod string;

use std::{borrow::Cow, ops::Add};

use iced_x86::{Instruction, Mnemonic, OpKind, Register};
use pe_parser::{pe::PortableExecutable, section::SectionHeader};

use crate::addr::Addr;

#[derive(Clone, Copy)]
pub struct SectionData<'a> {
    pub address: Addr,
    pub data: &'a [u8],
}

impl<'a> std::fmt::Debug for SectionData<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SectionData")
            .field("address", &self.address)
            .field("size", &format_args!("{:x}", self.data.len()))
            .finish()
    }
}

impl<'a> SectionData<'a> {
    pub fn contains(&self, addr: Addr) -> bool {
        let start = self.address.0 as u64;
        let range = start..(start + self.data.len() as u64);
        range.contains(&(addr.0 as _))
    }

    pub fn slice(&self, addr: Addr, size: usize) -> &'a [u8] {
        let index = self.index(addr);
        &self.data[index..index + size]
    }

    fn index(&self, addr: Addr) -> usize {
        usize::try_from(addr.0).unwrap() - usize::try_from(self.address.0).unwrap()
    }

    pub fn slice_cstr(&self, addr: Addr) -> Option<&'a std::ffi::CStr> {
        if !self.contains(addr) {
            return None;
        }

        let index = self.index(addr);
        std::ffi::CStr::from_bytes_until_nul(&self.data[index..]).ok()
    }

    pub fn slice_to_end(&self, addr: Addr) -> &'a [u8] {
        let index = self.index(addr);
        &self.data[index..]
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn read_u32_le(&self, addr: Addr) -> u32 {
        u32::from_le_bytes(self.slice(addr, 4).try_into().unwrap())
    }

    pub fn to_range(&self) -> std::ops::Range<Addr> {
        self.address..(self.address + Addr(self.data.len().try_into().unwrap()))
    }
}

#[derive(Debug)]
pub struct Sections<'a> {
    pub image_base: u32,
    pub text: SectionData<'a>,
    pub rdata: SectionData<'a>,
    pub data: SectionData<'a>,
}

impl<'a> Sections<'a> {
    pub fn slice(&self, addr: Addr, size: u32) -> Option<&'a [u8]> {
        for section in self.sections() {
            if section.contains(addr) {
                return Some(section.slice(addr, size as _));
            }
        }
        None
    }

    pub fn sections(&self) -> [&SectionData<'a>; 3] {
        [&self.text, &self.rdata, &self.data]
    }

    pub fn slice_cstr(&self, addr: Addr) -> Option<&'a std::ffi::CStr> {
        for section in self.sections() {
            if let Some(cstr) = section.slice_cstr(addr) {
                return Some(cstr);
            }
        }
        None
    }

    pub fn read_u32_le(&self, addr: Addr) -> Option<u32> {
        for section in self.sections() {
            if section.contains(addr) {
                return Some(section.read_u32_le(addr));
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct Binary<'a> {
    pub entry_point: Addr,
    pub sections: Sections<'a>,
    pub imports: Vec<ImportTableLib>,
    pub rdata_objects: Vec<DataObject>,
    pub data_objects: Vec<DataObject>,
}

pub fn get_section_bytes<'a>(
    pe_bytes: &'a [u8],
    image_base: u32,
    section: &SectionHeader,
) -> SectionData<'a> {
    let start = section.pointer_to_raw_data as usize;
    let size = u32::min(section.size_of_raw_data, section.virtual_size) as usize;

    let data = &pe_bytes[start..start + size];
    SectionData {
        address: Addr(image_base + section.virtual_address),
        data,
    }
}

pub fn parse_sections<'a>(pe_bytes: &'a [u8], pe: &PortableExecutable) -> Option<Sections<'a>> {
    let image_base = pe.optional_header_32?.image_base;

    let mut text = Option::None;
    let mut rdata = Option::None;
    let mut data = Option::None;

    for section in &pe.section_table {
        let Ok(name) = std::ffi::CStr::from_bytes_until_nul(&section.name) else {
            continue;
        };

        match name.to_bytes() {
            b".text" => text = Some(get_section_bytes(pe_bytes, image_base, section)),
            b".rdata" => rdata = Some(get_section_bytes(pe_bytes, image_base, section)),
            b".data" => data = Some(get_section_bytes(pe_bytes, image_base, section)),
            _ => {}
        }
    }

    Some(Sections {
        image_base,
        text: text?,
        rdata: rdata?,
        data: data?,
    })
}

pub fn parse_binary<'a>(pe_bytes: &'a [u8], pe: &PortableExecutable) -> Option<Binary<'a>> {
    let header = pe.optional_header_32.unwrap();

    let entry_point = Addr(header.image_base + header.address_of_entry_point);
    let sections = parse_sections(pe_bytes, pe)?;
    let imports = parse_import_table(&sections, pe)?;
    let rdata_objects = collect_data_objects(&sections, sections.rdata);
    let data_objects = collect_data_objects(&sections, sections.data);

    Some(Binary {
        entry_point,
        sections,
        imports,
        rdata_objects,
        data_objects,
    })
}

#[repr(C)]
#[derive(Debug)]
pub struct ImageImportDescriptor {
    pub addr: Addr,
    pub original_first_thunk: u32,
    pub time_date_stamp: u32,
    pub forwarder_chain: u32,
    pub name: u32,
    pub first_thunk: u32,
    pub size: usize,
}

pub fn parse_image_import_descriptor(
    mut data: &[u8],
    mut addr: Addr,
) -> Vec<ImageImportDescriptor> {
    let mut res = vec![];
    while data.len() >= 20 {
        let descriptor = ImageImportDescriptor {
            addr,
            original_first_thunk: u32::from_le_bytes(data[0..4].try_into().unwrap()),
            time_date_stamp: u32::from_le_bytes(data[4..8].try_into().unwrap()),
            forwarder_chain: u32::from_le_bytes(data[8..12].try_into().unwrap()),
            name: u32::from_le_bytes(data[12..16].try_into().unwrap()),
            first_thunk: u32::from_le_bytes(data[16..20].try_into().unwrap()),
            size: 20,
        };
        res.push(descriptor);

        data = &data[20..];
        addr += 20;
    }

    res
}

#[derive(Debug)]
pub struct ReadString {
    pub address: Addr,
    pub value: String,
}

#[derive(Debug)]
pub struct ImportTableThunk {
    pub name: ReadString,
    pub descriptor_addr: Addr,
    pub descriptor_size: usize,
    pub target_addr: Addr,
}

#[derive(Debug)]
pub struct ImportTableLib {
    pub descriptor_addr: Addr,
    pub name: ReadString,
    pub size: usize,
    pub thunks: Vec<ImportTableThunk>,
}

pub fn parse_import_table(
    sections: &Sections<'_>,
    pe: &PortableExecutable,
) -> Option<Vec<ImportTableLib>> {
    let image_base = sections.image_base;
    let import_table = pe.optional_header_32?.data_directories.import_table;
    let addr = Addr(image_base + import_table.virtual_address);

    let table = sections
        .slice(addr as _, import_table.size as _)
        .expect("failed to find import table");

    let mut res = vec![];
    for descriptor in parse_image_import_descriptor(table, addr) {
        if descriptor.name == 0 {
            res.push(ImportTableLib {
                descriptor_addr: descriptor.addr,
                name: ReadString {
                    address: Addr(0),
                    value: String::new(),
                },
                size: descriptor.size,
                thunks: vec![],
            });
            break;
        }

        let libname_addr = Addr(image_base + descriptor.name);
        let libname = sections
            .slice_cstr(libname_addr)
            .expect("failed to read import lib name")
            .to_str()
            .expect("import lib name is not utf8")
            .to_string();
        println!(">> Libname at {libname_addr} {libname}");

        let mut original_first_thunk = Addr(descriptor.original_first_thunk + image_base);
        let mut target_thunk = Addr(descriptor.first_thunk + image_base);

        let mut thunks = vec![];
        while let Some(ptr) = sections.read_u32_le(original_first_thunk) {
            if ptr == 0 {
                thunks.push(ImportTableThunk {
                    name: ReadString {
                        address: Addr(0),
                        value: String::new(),
                    },
                    descriptor_size: 4,
                    descriptor_addr: original_first_thunk,
                    target_addr: target_thunk,
                });
                break;
            }

            let is_ordinal = ptr & 0x80000000 > 0;
            if is_ordinal {
                panic!("Unsupported is_ordinal");
            }

            let ptr = Addr((ptr & 0x7FFFFFFF) + image_base);
            // +2 because structure looks like:
            // typedef struct {
            //     WORD Hint; // Unused
            //     BYTE Name[]; // ASCII null-terminated
            // } IMAGE_IMPORT_BY_NAME;
            let name_addr = ptr + 2;
            let funcname = sections
                .slice_cstr(name_addr)
                .expect("failed to read thunk name")
                .to_str()
                .expect("string name is not utf8")
                .to_string();

            thunks.push(ImportTableThunk {
                name: ReadString {
                    address: name_addr,
                    value: funcname,
                },
                descriptor_size: 4,
                descriptor_addr: original_first_thunk,
                target_addr: target_thunk,
            });

            original_first_thunk += 4;
            target_thunk += 4;
        }

        res.push(ImportTableLib {
            descriptor_addr: descriptor.addr,
            size: descriptor.size,
            name: ReadString {
                address: libname_addr,
                value: libname,
            },
            thunks,
        });
    }

    Some(res)
}

pub fn operand_signature_full(instr: &Instruction, index: u32) -> String {
    let kind = instr.op_kind(index);

    match kind {
        OpKind::Register => "REG".to_string(),

        OpKind::Immediate8 => format!("{:?}({})", kind, instr.immediate8()),
        OpKind::Immediate16 => format!("{:?}({})", kind, instr.immediate16()),
        OpKind::Immediate32 => format!("{:?}({})", kind, instr.immediate32()),
        OpKind::Immediate64 => format!("{:?}({})", kind, instr.immediate64()),
        OpKind::Immediate8_2nd => format!("{:?}({})", kind, instr.immediate8_2nd()),
        OpKind::Immediate8to16 => format!("{:?}({})", kind, instr.immediate8to16()),
        OpKind::Immediate8to32 => format!("{:?}({})", kind, instr.immediate8to32()),
        OpKind::Immediate8to64 => format!("{:?}({})", kind, instr.immediate8to64()),
        OpKind::Immediate32to64 => format!("{:?}({})", kind, instr.immediate32to64()),

        OpKind::Memory => {
            let base = instr.memory_base();
            let index = instr.memory_index();
            let scale = instr.memory_index_scale();
            let disp = instr.memory_displacement32();
            let segment = instr.memory_segment();

            format!(
                "MEM(base={:?},index={:?},scale={},disp={},segment={:?})",
                base, index, scale, disp, segment,
            )
        }

        OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64 => {
            "NEAR_BRANCH".to_string()
        }

        OpKind::FarBranch16 | OpKind::FarBranch32 => "FAR_BRANCH".to_string(),

        _ => format!("{:?}", kind),
    }
}

pub fn instruction_signature_full(instr: &iced_x86::Instruction) -> String {
    let mut ops = Vec::new();
    for i in 0..instr.op_count() {
        ops.push(operand_signature_full(instr, i));
    }
    format!("{:?}({})", instr.mnemonic(), ops.join(","))
}

pub fn operand_signature(instr: &Instruction, index: u32) -> Cow<'static, str> {
    let kind = instr.op_kind(index);

    match kind {
        OpKind::Register => Cow::Borrowed("REG"),

        OpKind::Immediate8
        | OpKind::Immediate16
        | OpKind::Immediate32
        | OpKind::Immediate64
        | OpKind::Immediate8_2nd
        | OpKind::Immediate8to16
        | OpKind::Immediate8to32
        | OpKind::Immediate8to64
        | OpKind::Immediate32to64 => Cow::Borrowed("IMM"),

        OpKind::Memory => Cow::Borrowed("MEM"),

        OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64 => {
            Cow::Borrowed("NEAR_BRANCH")
        }

        OpKind::FarBranch16 | OpKind::FarBranch32 => Cow::Borrowed("FAR_BRANCH"),

        _ => Cow::Owned(format!("{:?}", kind)),
    }
}

pub fn instruction_signature(instr: &iced_x86::Instruction) -> String {
    let mut ops = Vec::new();
    for i in 0..instr.op_count() {
        ops.push(operand_signature(instr, i));
    }
    format!("{:?}({})", instr.mnemonic(), ops.join(","))
}

pub fn extract_call_address(instr: &Instruction) -> Option<u64> {
    if instr.mnemonic() == Mnemonic::Call && instr.op_count() > 0 {
        // Check operand type
        match instr.op_kind(0) {
            OpKind::Memory => {
                // Absolute address: base = None, index = None
                let base = instr.memory_base();
                let index = instr.memory_index();
                let disp = instr.memory_displacement32();

                if base == Register::None && index == Register::None {
                    // This is a static memory address call
                    Some(disp as _)
                } else {
                    None
                }
            }
            OpKind::Immediate32 | OpKind::Immediate64 => {
                // Direct call to immediate (relative calls)
                // Could also handle if you want actual target addresses
                let imm = instr.immediate(0);
                Some(imm)
            }

            OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64 => {
                Some(instr.near_branch_target()) // already absolute
            }

            OpKind::FarBranch16 => {
                Some(instr.far_branch16().into()) // absolute, but includes selector logic
            }

            OpKind::FarBranch32 => {
                Some(instr.far_branch32().into()) // absolute, but includes selector logic
            }
            _ => None,
        }
    } else {
        None
    }
}

#[derive(Debug)]
pub enum DataObjectKind {
    Utf8String(String),
    Utf16String(String),
    FunctionsRef(Addr),
    RdataRef(Addr),
    DataRef(Addr),
    Const(u32),
}

#[derive(Debug)]
pub struct DataObject {
    pub addr: Addr,
    pub kind: DataObjectKind,
}

pub fn collect_data_objects(sections: &Sections<'_>, section: SectionData<'_>) -> Vec<DataObject> {
    let mut res = Vec::with_capacity(sections.rdata.len() / 4);

    let mut addr = section.address;
    let mut data = section.data;

    while !data.is_empty() {
        if let Some((s, size)) = string::read_utf8_or_win_1252(data) {
            res.push(DataObject {
                addr,
                kind: DataObjectKind::Utf8String(s),
            });
            // It seems like string have padding after them
            let size = size.next_multiple_of(2).min(data.len());
            addr += size as u32;
            data = &data[size..];
            continue;
        }

        // 2. UTF-16 string?
        if let Some((s, size)) = string::read_utf16_string(data) {
            res.push(DataObject {
                addr,
                kind: DataObjectKind::Utf16String(s),
            });

            let size = size.next_multiple_of(2).min(data.len());
            addr += size as u32;
            data = &data[size..];
            continue;
        }

        if addr.0 % 4 == 0 {
            let u32 = data[..4].try_into().unwrap();
            let u32 = u32::from_le_bytes(u32);
            let as_addr = Addr(u32);

            let kind = match as_addr {
                addr if sections.rdata.contains(addr) => DataObjectKind::RdataRef(addr),
                addr if sections.data.contains(addr) => DataObjectKind::DataRef(addr),
                addr if sections.text.contains(addr) => DataObjectKind::FunctionsRef(addr),
                _ => DataObjectKind::Const(u32),
            };

            res.push(DataObject { addr, kind });
        }

        data = &data[2..];
        addr += 2;
    }

    res
}
