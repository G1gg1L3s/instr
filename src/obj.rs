use std::{collections::BTreeMap, sync::Arc};

use crate::{ImportTableLib, addr::Addr, string::DataStringType};

#[derive(Debug, Clone)]
pub struct ImportThunk {
    pub lib: Arc<str>,
    pub func: Arc<str>,
    pub descriptor: Addr,
    pub is_terminating: bool,
}

#[derive(Debug, Clone)]
pub struct ImportLibDescriptor {
    pub lib: Arc<str>,
    pub size: usize,
}

#[derive(Debug, Clone)]
pub struct ImportFuncDescriptor {
    pub lib: Arc<str>,
    pub func: Arc<str>,
    pub size: usize,
    pub lib_descriptor: Addr,
    pub target_addr: Addr,
}

#[derive(Clone)]
pub enum ObjectTyp {
    ImportThunk(ImportThunk),
    ImportLibDescriptor(ImportLibDescriptor),
    ImportFuncDescriptor(ImportFuncDescriptor),
    String(DataStringType),
}

impl std::fmt::Debug for ObjectTyp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ImportThunk(arg) => arg.fmt(f),
            Self::ImportLibDescriptor(arg) => arg.fmt(f),
            Self::ImportFuncDescriptor(arg) => arg.fmt(f),
            Self::String(arg) => arg.fmt(f),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Object {
    addr: Addr,
    typ: ObjectTyp,
}

impl Object {
    pub fn new(addr: Addr, typ: ObjectTyp) -> Self {
        Self { addr, typ }
    }

    pub fn addr(&self) -> Addr {
        self.addr
    }

    pub fn len(&self) -> usize {
        match &self.typ {
            ObjectTyp::ImportThunk(_) => 4,
            ObjectTyp::ImportLibDescriptor(import_lib_descriptor) => import_lib_descriptor.size,
            ObjectTyp::ImportFuncDescriptor(import_func_descriptor) => import_func_descriptor.size,
            ObjectTyp::String(str) => str.len(),
        }
    }

    pub fn typ(&self) -> &ObjectTyp {
        &self.typ
    }
}

#[derive(Debug, Clone, Default)]
pub struct ObjDatabase {
    objects: BTreeMap<Addr, Object>,
}

impl ObjDatabase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, obj: Object) {
        self.objects.insert(obj.addr(), obj);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Object> {
        self.objects.values()
    }

    pub fn range<R: std::ops::RangeBounds<Addr>>(&self, range: R) -> impl Iterator<Item = &Object> {
        self.objects.range(range).map(|(_, v)| v)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Object> {
        self.objects.values_mut()
    }
}

pub fn fill_database_with_import(database: &mut ObjDatabase, lib: &ImportTableLib) {
    let libname: Arc<str> = lib.name.value.to_string().into();

    database.insert(Object::new(
        lib.descriptor_addr,
        ObjectTyp::ImportLibDescriptor(ImportLibDescriptor {
            lib: libname.clone(),
            size: lib.size,
        }),
    ));

    for thunk in &lib.thunks {
        let func_name: Arc<str> = thunk.name.value.to_string().into();

        database.insert(Object::new(
            thunk.descriptor_addr,
            ObjectTyp::ImportFuncDescriptor(ImportFuncDescriptor {
                lib: libname.clone(),
                func: func_name.clone(),
                size: thunk.descriptor_size,
                lib_descriptor: lib.descriptor_addr,
                target_addr: thunk.target_addr,
            }),
        ));

        database.insert(Object::new(
            thunk.target_addr,
            ObjectTyp::ImportThunk(ImportThunk {
                lib: libname.clone(),
                func: func_name.clone(),
                descriptor: thunk.descriptor_addr,
                is_terminating: is_well_known_exit(&libname, &func_name),
            }),
        ));
    }
}

fn is_well_known_exit(lib: &str, func: &str) -> bool {
    let known = [
        ("MSVCR71.dll", "exit"),
        ("MSVCR71.dll", "_exit"),
        ("MSVCR71.dll", "_cexit"),
        ("MSVCR71.dll", "_amsg_exit"),
    ];
    known.contains(&(lib, func))
}
