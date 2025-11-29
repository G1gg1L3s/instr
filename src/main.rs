use std::{collections::BTreeSet, ffi::CStr};

use iced_x86::{Decoder, Instruction, Mnemonic, OpKind, Register};
use pe_parser::{pe::PortableExecutable, section::SectionHeader};

#[derive(Clone, Copy)]
struct Addr(u32);

impl std::fmt::Debug for Addr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:x}", self.0)
    }
}

impl std::fmt::Display for Addr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:x}", self.0)
    }
}

impl std::ops::Add<Addr> for Addr {
    type Output = Addr;

    fn add(self, rhs: Self) -> Self::Output {
        Addr(self.0 + rhs.0)
    }
}

impl std::ops::Add<u32> for Addr {
    type Output = Addr;

    fn add(self, rhs: u32) -> Self::Output {
        Addr(self.0 + rhs)
    }
}

impl std::ops::AddAssign<u32> for Addr {
    fn add_assign(&mut self, rhs: u32) {
        self.0 += rhs;
    }
}

struct SectionData<'a> {
    address: Addr,
    data: &'a [u8],
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
    fn contains(&self, addr: Addr) -> bool {
        let start = self.address.0 as u64;
        let range = start..(start + self.data.len() as u64);
        range.contains(&(addr.0 as _))
    }

    fn slice(&self, addr: Addr, size: usize) -> &'a [u8] {
        let index = usize::try_from(addr.0).unwrap() - usize::try_from(self.address.0).unwrap();
        &self.data[index..index + size]
    }

    fn slice_cstr(&self, addr: Addr) -> Option<&'a std::ffi::CStr> {
        if !self.contains(addr) {
            return None;
        }

        let index = usize::try_from(addr.0).unwrap() - usize::try_from(self.address.0).unwrap();
        std::ffi::CStr::from_bytes_until_nul(&self.data[index..]).ok()
    }

    fn len(&self) -> usize {
        self.data.len()
    }
}

#[derive(Debug)]
struct Sections<'a> {
    image_base: u32,
    text: SectionData<'a>,
    rdata: SectionData<'a>,
    data: SectionData<'a>,
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

    fn sections(&self) -> [&SectionData<'a>; 3] {
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
                return Some(u32::from_le_bytes(
                    section.slice(addr, 4).try_into().unwrap(),
                ));
            }
        }
        None
    }
}

#[derive(Debug)]
struct Binary<'a> {
    sections: Sections<'a>,
    imports: Vec<ImportTableLib>,
}

fn get_section_bytes<'a>(
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

fn parse_sections<'a>(pe_bytes: &'a [u8], pe: &PortableExecutable) -> Option<Sections<'a>> {
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

fn parse_binary<'a>(pe_bytes: &'a [u8], pe: &PortableExecutable) -> Option<Binary<'a>> {
    let sections = parse_sections(pe_bytes, pe)?;
    let imports = parse_import_table(&sections, pe)?;

    Some(Binary { sections, imports })
}

#[repr(C)]
#[derive(Debug)]
struct ImageImportDescriptor {
    addr: Addr,
    original_first_thunk: u32,
    time_date_stamp: u32,
    forwarder_chain: u32,
    name: u32,
    first_thunk: u32,
}

fn parse_image_import_descriptor(mut data: &[u8], mut addr: Addr) -> Vec<ImageImportDescriptor> {
    let mut res = vec![];
    while data.len() >= 20 {
        let descriptor = ImageImportDescriptor {
            addr,
            original_first_thunk: u32::from_le_bytes(data[0..4].try_into().unwrap()),
            time_date_stamp: u32::from_le_bytes(data[4..8].try_into().unwrap()),
            forwarder_chain: u32::from_le_bytes(data[8..12].try_into().unwrap()),
            name: u32::from_le_bytes(data[12..16].try_into().unwrap()),
            first_thunk: u32::from_le_bytes(data[16..20].try_into().unwrap()),
        };
        res.push(descriptor);

        data = &data[20..];
        addr += 20;
    }

    res
}

#[derive(Debug)]
struct ReadString {
    address: Addr,
    value: String,
}

#[derive(Debug)]
struct ImportTableThunk {
    name: ReadString,
    descriptor_addr: Addr,
    target_addr: Addr,
}

#[derive(Debug)]
struct ImportTableLib {
    descriptor_addr: Addr,
    name: ReadString,
    thunks: Vec<ImportTableThunk>,
}

fn parse_import_table(
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
            break;
        }

        let libname_addr = Addr(image_base + descriptor.name);
        let libname = sections
            .slice_cstr(libname_addr)
            .expect("failed to read import lib name")
            .to_str()
            .expect("import lib name is not utf8")
            .to_string();

        let mut original_first_thunk = Addr(descriptor.original_first_thunk + image_base);
        let mut target_thunk = Addr(descriptor.first_thunk + image_base);

        let mut thunks = vec![];
        while let Some(ptr) = sections.read_u32_le(original_first_thunk) {
            if ptr == 0 {
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
                descriptor_addr: original_first_thunk,
                target_addr: target_thunk,
            });

            original_first_thunk += 4;
            target_thunk += 4;
        }

        res.push(ImportTableLib {
            descriptor_addr: descriptor.addr,
            name: ReadString {
                address: libname_addr,
                value: libname,
            },
            thunks,
        });
    }

    Some(res)
}

fn operand_signature(instr: &Instruction, index: u32) -> String {
    let kind = instr.op_kind(index);

    match kind {
        OpKind::Register => "REG".to_string(),

        OpKind::Immediate8
        | OpKind::Immediate16
        | OpKind::Immediate32
        | OpKind::Immediate64
        | OpKind::Immediate8_2nd
        | OpKind::Immediate8to16
        | OpKind::Immediate8to32
        | OpKind::Immediate8to64
        | OpKind::Immediate32to64 => "IMM".to_string(),

        OpKind::Memory => {
            let base = instr.memory_base();
            let index = instr.memory_index();
            let scale = instr.memory_index_scale();
            let disp = instr.memory_displacement32();

            // format!(
            //     "MEM(base={},index={},scale={},disp={})",
            //     base != Register::None,
            //     index != Register::None,
            //     scale != 1,
            //     disp != 0
            // )
            "MEM".to_string()
        }

        OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64 => {
            "NEAR_BRANCH".to_string()
        }

        OpKind::FarBranch16 | OpKind::FarBranch32 => "FAR_BRANCH".to_string(),

        _ => format!("{:?}", kind),
    }
}

fn instruction_signature(instr: &iced_x86::Instruction) -> String {
    let mut ops = Vec::new();
    for i in 0..instr.op_count() {
        ops.push(operand_signature(instr, i));
    }
    format!("{:?}({})", instr.mnemonic(), ops.join(","))
}

fn extract_call_address(instr: &Instruction) -> Option<u64> {
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
            _ => None,
        }
    } else {
        None
    }
}

#[derive(Debug)]
enum RDataObjectKind {
    Utf8String(String),
    Utf16String(String),
    FunctionsRef(Addr),
    RdataRef(Addr),
    DataRef(Addr),
    Const(u32),
}

#[derive(Debug)]
struct RDataObject {
    addr: Addr,
    kind: RDataObjectKind,
}

fn read_ascii_string(data: &[u8]) -> Option<(String, usize)> {
    let mut end = 0;
    while end < data.len() {
        if data[end] == 0 {
            let s = str::from_utf8(&data[..end]).unwrap();
            if s.len() >= 4 {
                return Some((s.to_string(), end + 1));
            } else {
                return None;
            }
        }
        if !(0x20..=0x7E).contains(&data[end]) {
            return None;
        }
        end += 1;
    }
    None
}

fn read_utf16_string(data: &[u8]) -> Option<(String, usize)> {
    if data.len() < 2 {
        return None;
    }
    if data[1] != 0 {
        return None;
    } // must start with LE ASCII-range wide char

    let mut end = 0;
    while end + 1 < data.len() {
        if data[end] == 0 && data[end + 1] == 0 {
            let units: Vec<u16> = data[..end]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let s = String::from_utf16(&units).unwrap();
            if s.len() >= 4 {
                return Some((s, end + 2));
            } else {
                return None;
            }
        }
        if data[end + 1] != 0 {
            return None;
        } // only ASCII-range UTF-16 allowed here
        end += 2;
    }
    None
}

fn collect_rdata_objects(sections: &Sections<'_>) -> Vec<RDataObject> {
    let mut res = Vec::with_capacity(sections.rdata.len() / 4);

    let mut addr = sections.rdata.address;
    let mut data = sections.rdata.data;

    while !data.is_empty() {
        if let Some((s, size)) = read_ascii_string(data) {
            res.push(RDataObject {
                addr,
                kind: RDataObjectKind::Utf8String(s),
            });
            // It seems like string have padding after them
            let size = size.next_multiple_of(4).min(data.len());
            addr += size as u32;
            data = &data[size..];
            continue;
        }

        // 2. UTF-16 string?
        if let Some((s, size)) = read_utf16_string(data) {
            res.push(RDataObject {
                addr,
                kind: RDataObjectKind::Utf16String(s),
            });

            let size = size.next_multiple_of(4).min(data.len());
            addr += size as u32;
            data = &data[size..];
            continue;
        }

        let u32 = data[..4].try_into().unwrap();
        let u32 = u32::from_le_bytes(u32);
        let as_addr = Addr(u32);

        let kind = match as_addr {
            addr if sections.rdata.contains(addr) => RDataObjectKind::RdataRef(addr),
            addr if sections.data.contains(addr) => RDataObjectKind::DataRef(addr),
            addr if sections.text.contains(addr) => RDataObjectKind::FunctionsRef(addr),
            _ => RDataObjectKind::Const(u32),
        };

        res.push(RDataObject { addr, kind });

        data = &data[4..];
        addr += 4;
    }

    res
}

fn main() {
    let data = std::fs::read("../../Barnyard/Barnyard.exe").unwrap();
    let pe = pe_parser::pe::parse_portable_executable(&data).unwrap();
    let binary = parse_binary(&data, &pe).unwrap();

    println!("Binary: {:#?}", binary);

    let robjects = collect_rdata_objects(&binary.sections);
    for obj in robjects {
        println!(">> {:?}", obj);
    }

    return;

    let mut decoder = Decoder::with_ip(
        32,
        binary.sections.text.data,
        binary.sections.text.address.0.into(),
        iced_x86::DecoderOptions::NONE,
    );

    let mut variants = BTreeSet::new();

    for instr in &mut decoder {
        let sig = instruction_signature(&instr);
        let instrformat = instr.to_string();
        eprintln!("0x{:x} {: <30} | {}", instr.ip(), instrformat, sig);
        variants.insert(sig);

        if let Some(addr) = extract_call_address(&instr) {
            eprintln!(
                ">> 0x{addr:x} (addr in .text: {})",
                binary.sections.text.contains(Addr(addr as _))
            );
        }
    }

    // ----- PRINT RESULTS -----
    println!("\nUnique instruction variants: {}", variants.len());
    for v in variants {
        println!("{}", v);
    }
}
