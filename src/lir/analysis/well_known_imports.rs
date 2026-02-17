mod c_parse;

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::{Arc, LazyLock},
};

use crate::{
    addr::Addr,
    lir::io::Io,
    obj::{Imported, ObjDatabase, ObjectTyp},
};

#[derive(Debug, Clone)]
pub struct CallingConvention {
    pub inputs: HashSet<Io>,
    pub outputs: HashSet<Io>,
    pub stack_adjust: u16,
}

pub struct ImportedFunction {
    pub addr: Addr,
    pub lib: Arc<str>,
    pub function: Arc<str>,
    pub calling_convention: CallingConvention,
}

pub struct WellKnownImports {
    imports: BTreeMap<Addr, ImportedFunction>,
}

impl WellKnownImports {
    pub fn new_known(db: &ObjDatabase) -> WellKnownImports {
        let mut imports = BTreeMap::new();
        for object in db.iter() {
            let ObjectTyp::Imported(Imported { thunk }) = object.typ() else {
                continue;
            };

            let thunk = db.get(*thunk).unwrap();
            let ObjectTyp::ImportThunk(thunk) = thunk.typ() else {
                unreachable!()
            };
            if thunk.func.is_empty() {
                continue;
            }

            let calling_convention = calling_convention_for(&thunk.lib, &thunk.func);

            imports.insert(
                object.addr(),
                ImportedFunction {
                    addr: object.addr(),
                    lib: Arc::clone(&thunk.lib),
                    function: Arc::clone(&thunk.func),
                    calling_convention,
                },
            );
        }
        Self { imports }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Addr, &ImportedFunction)> {
        self.imports.iter().map(|(k, v)| (*k, v))
    }
}

const ALL_DEFS: &str = r#"
stdcall LONG RegQueryValueExA(HKEY hKey,LPCSTR lpValueName,LPDWORD lpReserved,LPDWORD lpType,LPBYTE lpData,LPDWORD lpcbData);
stdcall LONG RegOpenKeyExA(HKEY hKey,LPCSTR lpSubKey,DWORD ulOptions,REGSAM samDesired,PHKEY phkResul);
stdcall LONG RegOpenKeyA(HKEY hKey,LPCSTR lpSubKey,PHKEY phkResult);
stdcall LONG RegCloseKey(HKEY hKey);

stdcall HRESULT DirectInput8Create(HINSTANCE hinst,DWORD dwVersion,REFIID riidltf,LPVOID * ppvOut,LPUNKNOWN punkOuter);
stdcall WINBOOL DeleteDC(HDC hdc);
stdcall WINBOOL ExtTextOutA(HDC hdc,int x,int y,UINT options,CONST RECT * lprect,LPCSTR lpString,UINT c,CONST INT * lpDx);
stdcall WINBOOL GetTextExtentPoint32A(HDC hdc,LPCSTR lpString,int c,LPSIZE psizl);

stdcall WINBOOL DeleteObject(HGDIOBJ ho);
stdcall UINT SetTextAlign(HDC hdc,UINT align);
stdcall COLORREF SetBkColor(HDC hdc,COLORREF color);
stdcall HDC CreateCompatibleDC(HDC hdc);

stdcall HBITMAP CreateDIBSection(HDC hdc,CONST BITMAPINFO * lpbmi,UINT usage,VOID ** ppvBits,HANDLE hSection,DWORD offset);
stdcall int SetMapMode(HDC hdc,int iMode);
stdcall int GetDeviceCaps(HDC hdc,int index);
stdcall HFONT CreateFontA(int cHeight,int cWidth,int cEscapement,int cOrientation,int cWeight,DWORD bItalic,DWORD bUnderline,DWORD bStrikeOut,DWORD iCharSet,DWORD iOutPrecision,DWORD iClipPrecision,DWORD iQuality,DWORD iPitchAndFamily,LPCSTR pszFaceName);
stdcall HGDI SelectObject(HDC hdc,HGDIOBJ h);
stdcall COLORREF SetTextColor(HDC hdc,COLORREF color);
stdcall HGDIOBJ GetStockObject(int i);

stdcall DWORD GetLastError (VOID);
stdcall VOID OutputDebugStringA (LPCSTR lpOutputString);
stdcall DWORD GetFileSizeFromContext (FIO_CONTEXT * pContext, DWORD * pcbFileSizeHigh);
stdcall HANDLE CreateFileW (LPCWSTR lpFileName, DWORD dwDesiredAccess, DWORD dwShareMode, LPSECURITY_ATTRIBUTES lpSecurityAttributes, DWORD dwCreationDisposition, DWORD dwFlagsAndAttributes, HANDLE hTemplateFile);
stdcall DWORD GetFileSize (HANDLE hFile, LPDWORD lpFileSizeHigh);
stdcall DWORD SetFilePointer (HANDLE hFile, LONG lDistanceToMove, PLONG lpDistanceToMoveHigh, DWORD dwMoveMethod);
stdcall HANDLE CreateFileA (LPCSTR lpFileName, DWORD dwDesiredAccess, DWORD dwShareMode, LPSECURITY_ATTRIBUTES lpSecurityAttributes, DWORD dwCreationDisposition, DWORD dwFlagsAndAttributes, HANDLE hTemplateFile);
stdcall HANDLE FindFirstFileA (LPCSTR lpFileName, LPWIN32_FIND_DATAA lpFindFileData);
stdcall WINBOOL GetFileSizeEx (HANDLE hFile, PLARGE_INTEGER lpFileSize);
stdcall WINBOOL GetFileTime (HANDLE hFile, LPFILETIME lpCreationTime, LPFILETIME lpLastAccessTime, LPFILETIME lpLastWriteTime);
stdcall WINBOOL CreateDirectoryA (LPCSTR lpPathName, LPSECURITY_ATTRIBUTES lpSecurityAttributes);
stdcall WINBOOL DeleteFileA (LPCSTR lpFileName);
stdcall WINBOOL FindNextFileA (HANDLE hFindFile, LPWIN32_FIND_DATAA lpFindFileData);
stdcall WINBOOL ReadFile (HANDLE hFile, LPVOID lpBuffer, DWORD nNumberOfBytesToRead, LPDWORD lpNumberOfBytesRead, LPOVERLAPPED lpOverlapped);
stdcall WINBOOL WriteFile (HANDLE hFile, LPCVOID lpBuffer, DWORD nNumberOfBytesToWrite, LPDWORD lpNumberOfBytesWritten, LPOVERLAPPED lpOverlapped);
stdcall WINBOOL CloseHandle (HANDLE hObject);
stdcall HMODULE GetModuleHandleA (LPCSTR lpModuleName);
stdcall FARPROC GetProcAddress (HMODULE hModule, LPCSTR lpProcName);
stdcall DWORD GetModuleFileNameA (HMODULE hModule, LPSTR lpFilename, DWORD nSize);
stdcall WINBOOL UnmapViewOfFile (LPCVOID lpBaseAddress);
stdcall LPVOID MapViewOfFile (HANDLE hFileMappingObject, DWORD dwDesiredAccess, DWORD dwFileOffsetHigh, DWORD dwFileOffsetLow, SIZE_T dwNumberOfBytesToMap);
stdcall DWORD GetConsoleTitleA(LPSTR lpConsoleTitle,DWORD nSize);
stdcall WINBOOL SetConsoleTitleA(LPCSTR lpConsoleTitle);
stdcall VOID GlobalMemoryStatus (LPMEMORYSTATUS lpBuffer);
stdcall int MulDiv (int nNumber, int nNumerator, int nDenominator);
stdcall HANDLE CreateSemaphoreA (LPSECURITY_ATTRIBUTES lpSemaphoreAttributes, LONG lInitialCount, LONG lMaximumCount, LPCSTR lpName);
stdcall HANDLE CreateFileMappingA (HANDLE hFile, LPSECURITY_ATTRIBUTES lpFileMappingAttributes, DWORD flProtect, DWORD dwMaximumSizeHigh, DWORD dwMaximumSizeLow, LPCSTR lpName);
stdcall HMODULE LoadLibraryA (LPCSTR lpLibFileName);
stdcall VOID GetStartupInfoA (LPSTARTUPINFOA lpStartupInfo);
stdcall DWORD GetPrivateProfileStringA (LPCSTR lpAppName, LPCSTR lpKeyName, LPCSTR lpDefault, LPSTR lpReturnedString, DWORD nSize, LPCSTR lpFileName);
stdcall WINBOOL QueryPerformanceCounter (LARGE_INTEGER * lpPerformanceCount);
stdcall WINBOOL QueryPerformanceFrequency (LARGE_INTEGER * lpFrequency);
stdcall VOID ExitProcess (UINT uExitCode);
stdcall WINBOOL SetThreadPriorityBoost (HANDLE hThread, WINBOOL bDisablePriorityBoost);
stdcall WINBOOL TerminateThread (HANDLE hThread, DWORD dwExitCode);
stdcall DWORD GetCurrentProcessId (VOID);
stdcall DWORD GetCurrentThreadId (VOID);
stdcall WINBOOL IsProcessorFeaturePresent (DWORD ProcessorFeature);
stdcall WINBOOL SetThreadPriority (HANDLE hThread, int nPriority);
stdcall DWORD ResumeThread (HANDLE hThread);
stdcall VOID GetSystemTimeAsFileTime (LPFILETIME lpSystemTimeAsFileTime);
stdcall WINBOOL GetVersionExA (LPOSVERSIONINFOA lpVersionInformation);
stdcall DWORD GetTickCount (VOID);
stdcall int WideCharToMultiByte (UINT CodePage, DWORD dwFlags, LPCWCH lpWideCharStr, int cchWideChar, LPSTR lpMultiByteStr, int cbMultiByte, LPCCH lpDefaultChar, LPBOOL lpUsedDefaultChar);
stdcall VOID EnterCriticalSection (LPCRITICAL_SECTION lpCriticalSection);
stdcall VOID LeaveCriticalSection (LPCRITICAL_SECTION lpCriticalSection);
stdcall VOID DeleteCriticalSection (LPCRITICAL_SECTION lpCriticalSection);
stdcall WINBOOL ReleaseSemaphore (HANDLE hSemaphore, LONG lReleaseCount, LPLONG lpPreviousCount);
stdcall WINBOOL ReleaseMutex (HANDLE hMutex);
stdcall VOID InitializeCriticalSection (LPCRITICAL_SECTION lpCriticalSection);
stdcall DWORD WaitForSingleObject (HANDLE hHandle, DWORD dwMilliseconds);
stdcall HANDLE CreateMutexA (LPSECURITY_ATTRIBUTES lpMutexAttributes, WINBOOL bInitialOwner, LPCSTR lpName);
stdcall VOID Sleep (DWORD dwMilliseconds);
stdcall LPSTR GetCommandLineA (VOID);
stdcall DWORD GetCurrentDirectoryA (DWORD nBufferLength, LPSTR lpBuffer);

cdecl _onexit_t _onexit(_onexit_t _Func);
cdecl char  _itoa(int _Value,char * _Dest,int _Radix);
cdecl char  _strtime(char * _Buffer);
cdecl char  strstr(const char * _Str,const char * _SubStr);
cdecl char _strlwr(char * _String);
cdecl char _ultoa(unsigned long _Value,char * _Dest,int _Radix);
cdecl char strchr(const char * _Str,int _Val);
cdecl char strrchr(const char * _Str,int _Ch);
cdecl double atof(const char * _String);
cdecl double floor(double _X);
cdecl FILE fopen(const char * _Filename,const char * _Mode);
cdecl int _findclose(intptr_t _FindHandle);
cdecl int _ismbblead(unsigned int _Ch);
cdecl int _mkdir(const char * _Path);
cdecl int _stricmp(const char * _Str1,const char * _Str2);
cdecl int _strnicmp(const char * _Str1,const char * _Str2,size_t _MaxCount);
cdecl int _vsnprintf(char * _Dest,size_t _Count,const char * _Format,va_list _Args);
cdecl int _vsnwprintf(wchar_t * _Dest,size_t _Count,const wchar_t * _Format,va_list _Args);
cdecl int _XcptFilter(unsigned long _ExceptionNum,struct _EXCEPTION_POINTERS * _ExceptionPtr);
cdecl int fclose(FILE * _File);
cdecl int isdigit(int _C);
cdecl int isspace(int _C);
cdecl int iswspace(wint_t ch);
cdecl int sprintf(char * _Dest,const char * _Format,...);
cdecl int strncmp(const char * _Str1,const char * _Str2,size_t _MaxCount);
cdecl int tolower(int _C);
cdecl int toupper(int _C);
cdecl int vsprintf(char * _Dest,const char * _Format,va_list _Args);
cdecl int wcscmp(const wchar_t * _Str1,const wchar_t * _Str2);
cdecl int wcsncmp(const wchar_t * _Str1,const wchar_t * _Str2,size_t _MaxCount);
cdecl long atol(const char * _Str);
cdecl void* memmove(void * _Dst,const void * _Src,size_t _MaxCount);
cdecl size_t fread(void *  _DstBuf,size_t _ElementSize,size_t _Count,FILE * _File);
cdecl size_t fwrite(const void * _Str,size_t _Size,size_t _Count,FILE * _File);
cdecl size_t wcslen(const wchar_t * _Str);
cdecl struct tm * localtime(const time_t * _Time);
cdecl time_t time(time_t * _Time);
cdecl uintptr_t _beginthreadex(void * _Security,unsigned _StackSize,unsigned void * _StartAddress,void * _ArgList,unsigned _InitFlag,unsigned * _ThrdAddr);
cdecl void  malloc(size_t _Size);
cdecl void _endthreadex(unsigned _Retval);
cdecl void calloc(size_t _NumOfElements,size_t _SizeOfElements);
cdecl void free(void * _Memory);
cdecl void longjmp(jmp_buf _Buf,int _Value);
cdecl wchar_t  _wcslwr(wchar_t * _String);
cdecl wchar_t  _wcsupr(wchar_t * _String);
cdecl wchar_t  wcschr(const wchar_t * _Str,wchar_t _Ch);
cdecl wchar_t  wcscpy(wchar_t * _Dest,const wchar_t * _Source);
cdecl wchar_t  wcsncpy(wchar_t * _Dest,const wchar_t * _Source,size_t _Count);
cdecl wchar_t  wcsstr(const wchar_t * _Str,const wchar_t * _SubStr);
cdecl unsigned int _controlfp (unsigned int unNew, unsigned int unMask);
cdecl void __set_app_type(int app_type);
cdecl void _purecall(void);
cdecl DWORD __CxxFrameHandler(PEXCEPTION_RECORD rec, EXCEPTION_REGISTRATION_RECORD* frame, PCONTEXT context, EXCEPTION_REGISTRATION_RECORD** dispatch);
cdecl void exit(int code);
cdecl voidfpu _CIfmod(voidfpu);
cdecl int _except_handler3(PEXCEPTION_RECORD rec, MSVCRT_EXCEPTION_FRAME* frame, PCONTEXT context, void* dispatcher);
cdecl intptr_t _findfirst(const char * fspec, struct _finddata_t* ft);
cdecl voidfpu _CIacos(voidfpu);
cdecl int _finite(double num);
cdecl int _setjmp3(OUT jmp_buf env, int count, ...);
cdecl intfpu _ftol(voidfpu);
cdecl double _CIpow(voidfpu);
cdecl voidfpu _CIasin(voidfpu);
cdecl char* strncpy(char* dst,const char* src,size_t len);
cdecl int vswprintf(wchar_t * buffer, size_t size, const wchar_t * format, va_list args);
cdecl void __security_error_handler(int code, void * data);
cdecl void ?terminate@@YAXXZ(void);
cdecl _onexit_t __dllonexit(_onexit_t func, _onexit_t ** start, _onexit_t ** end);
cdecl void _c_exit(void);
cdecl void _exit(int exitcode);
cdecl void _cexit(void);
cdecl void _amsg_exit(int errnum);
cdecl void __getmainargs(int * argc, char *** argc, char *** env, int expand_wildcards, int* new_mode);
cdecl void _initterm(_INITTERMFUN * start,_INITTERMFUN * end);
cdecl void __setusermatherr(MSVCRT_matherr_func func);
cdecl void _adjust_fdiv(void);
cdecl unsigned int* __p__commode(void);
cdecl int* __p__fmode(void);

symbol _acmdln;

stdcall signed char _FSOUND_Stream_SetMode@8(void* stream, unsigned int mode);
stdcall signed char _FSOUND_SetFrequency@8(int channel, int frequency);
stdcall signed char _FSOUND_3D_SetDopplerFactor@4(float factor);
stdcall signed char _FSOUND_3D_SetRolloffFactor@4(float factor);
stdcall signed char _FSOUND_3D_SetDistanceFactor@4(float factor);
stdcall signed char _FSOUND_SetBufferSize@4(int len_ms);
stdcall signed char _FSOUND_SetMinHardwareChannels@4(int min);
stdcall void _FSOUND_Close@0(void);
stdcall int _FSOUND_GetMaxChannels@0(void);
stdcall signed char _FSOUND_GetNumHWChannels@12(int* total, int* free, int* used);
stdcall signed char _FSOUND_Init@12(int mixrate, int maxsoftwarechannels, unsigned int flags);
stdcall signed char _FSOUND_GetMemoryStats@8(int* currentalloced, int* maxalloced);
stdcall signed char _FSOUND_Stream_SetTime@8(void* stream, int ms);
stdcall signed char _FSOUND_StopSound@4(int channel);
stdcall signed char _FSOUND_GetReserved@4(int channel);
stdcall unsigned int _FSOUND_Stream_GetMode@4(void* stream);
stdcall signed char _FSOUND_SetPriority@8(int channel, int priority);
stdcall int _FSOUND_Stream_GetNumSubStreams@4(void* stream);
stdcall int _FSOUND_PlaySoundEx@16(int channel, void* sample, void* dsp, int paused);
stdcall signed char _FSOUND_Sample_SetMode@8(void* sample, unsigned int mode);
stdcall unsigned int _FSOUND_Sample_GetMode@4(void* sample);
stdcall void _FSOUND_Update@0(void);
stdcall signed char _FSOUND_3D_Listener_SetAttributes@32(float* pos, float* vel, float fx, float fy, float fz, float tx, float ty, float tz);
stdcall signed char _FSOUND_3D_SetMinMaxDistance@12(int channel, float min, float max);
stdcall char* _FSOUND_Sample_GetName@4(void* sample);
stdcall int _FSOUND_Stream_GetLengthMs@4(void* stream);
stdcall signed char _FSOUND_IsPlaying@4(int channel);
stdcall signed char _FSOUND_Stream_SetSubStream@8(void* stream, int index);
stdcall int _FSOUND_Stream_PlayEx@16(int channel, void* stream, void* dsp, int paused);
stdcall int _FSOUND_GetNumSubChannels@4(int channel);
stdcall signed char _FSOUND_SetPan@8(int channel, int pan);
stdcall signed char _FSOUND_SetVolume@8(int channel, int vol);
stdcall signed char _FSOUND_Stream_SetBufferSize@4(int ms);
stdcall void* _FSOUND_Stream_Open@16(char* name_or_data, unsigned int mode, int offset, int length);
stdcall signed char _FSOUND_SetPaused@8(int channel, int paused);
stdcall signed char _FSOUND_SetReserved@8(int channel, int reserved);
stdcall int _FSOUND_GetSubChannel@8(int channel, int index);
stdcall int _FSOUND_Stream_GetOpenState@4(void* stream);
stdcall signed char _FSOUND_Stream_Stop@4(void* stream);
stdcall signed char _FSOUND_Stream_Close@4(void* stream);
stdcall int _FMUSIC_GetOpenState@4(void* module);
stdcall signed char _FSOUND_Sample_GetDefaults@20(void* sample, int* freq, int* vol, int* pan, int* priority, unsigned int* mode);
stdcall void* _FMUSIC_GetSample@8(void* module, int index);
stdcall int _FMUSIC_GetNumSamples@4(void* module);
stdcall void* _FMUSIC_LoadSongEx@24(char* name_or_data, int offset, int length, unsigned int mode, int* samplelist, int samplelistnum);
stdcall signed char _FMUSIC_FreeSong@4(void* module);
stdcall int _FSOUND_GetOutputHandle@0(void);
stdcall int _FSOUND_GetOutput@0(void);
stdcall signed char _FSOUND_3D_SetAttributes@12(int channel, float* pos, float* vel);
stdcall void* _FSOUND_GetCurrentSample@4(int channel);
stdcall int _FSOUND_Stream_GetTime@4(void* stream);

stdcall HACCEL CreateAcceleratorTableA(LPACCEL paccel,int cAccel);
stdcall WINBOOL GetWindowRect(HWND hWnd,LPRECT lpRect);
stdcall WINBOOL ShowWindow(HWND hWnd,int nCmdShow);
stdcall WINBOOL UpdateWindow(HWND hWnd);
stdcall WINBOOL DestroyAcceleratorTable(HACCEL hAccel);
stdcall WINBOOL SetCursorPos(int X,int Y);
stdcall HICON LoadIconA(HINSTANCE hInstance,LPCSTR lpIconName);
stdcall LRESULT SendMessageA(HWND hWnd,UINT Msg,WPARAM wParam,LPARAM lParam);
stdcall int MessageBoxA(HWND hWnd,LPCSTR lpText,LPCSTR lpCaption,UINT uType);
stdcall WINBOOL GetMonitorInfoA(HMONITOR hMonitor,LPMONITORINFO lpmi);
stdcall WINBOOL GetClientRect(HWND hWnd,LPRECT lpRect);
stdcall HWND GetDesktopWindow(VOID);
stdcall WINBOOL GetCursorPos(LPPOINT lpPoint);
stdcall LONG GetWindowLongA(HWND hWnd,int nIndex);
stdcall WINBOOL SystemParametersInfoA(UINT uiAction,UINT uiParam,PVOID pvParam,UINT fWinIni);
stdcall WINBOOL DestroyWindow(HWND hWnd);
stdcall WINBOOL PeekMessageA(LPMSG lpMsg,HWND hWnd,UINT wMsgFilterMin,UINT wMsgFilterMax,UINT wRemoveMsg);
stdcall HCURSOR LoadCursorA(HINSTANCE hInstance,LPCSTR lpCursorName);
stdcall HWND CreateWindowExA(DWORD dwExStyle,LPCSTR lpClassName,LPCSTR lpWindowName,DWORD dwStyle,int X,int Y,int nWidth,int nHeight,HWND hWndParent,HMENU hMenu,HINSTANCE hInstance,LPVOID lpParam);
stdcall WINBOOL EnableWindow(HWND hWnd,WINBOOL bEnable);
stdcall LRESULT DispatchMessageA(CONST MSG* lpMsg);
stdcall WINBOOL TranslateMessage(CONST MSG* lpMsg);
stdcall LONG SetWindowLongA(HWND hWnd,int nIndex,LONG dwNewLong);
stdcall WINBOOL GetMessageA(LPMSG lpMsg,HWND hWnd,UINT wMsgFilterMin,UINT wMsgFilterMax);
stdcall HWND FindWindowA(LPCSTR lpClassName,LPCSTR lpWindowName);
stdcall WINBOOL UnregisterClassA(LPCSTR lpClassName, HINSTANCE hInstance);
stdcall ATOM RegisterClassA(CONST WNDCLASSA * lpWndClass);
stdcall LRESULT DefWindowProcA(HWND hWnd, UINT Msg, WPARAM wParam, LPARAM lParam);
stdcall WINBOOL SetWindowPos(HWND hWnd, HWND hWndInsertAfter, int X, int Y, int cx, int cy, UINT uFlags);

stdcall DWORD timeGetTime(void);

# https://github.com/TheSuperHackers/bink-sdk-stub/blob/master/bink.h
stdcall int _BinkWait@4(HBINK handle);
stdcall int _BinkDoFrame@4(HBINK handle);
stdcall int _BinkCopyToBuffer@28(HBINK handle, void* dest, int destpitch, unsigned int destheight, unsigned int destx, unsigned int desty, unsigned int flags);
stdcall void _BinkNextFrame@4(HBINK handle);
stdcall void _BinkSetSoundOnOff@8(HBINK bink, int onoff);
stdcall void _BinkClose@4(HBINK handle);
stdcall void _BinkPause@8(HBINK bink, int pause);
stdcall void _BinkSetMemory@8(void* malloc_callback, void* free_callback);
stdcall void* _BinkOpenDirectSound@4(LPDIRECTSOUND directsound);
stdcall void _BinkOpenWaveOut@4(UINT deviceid);
stdcall HBINK _BinkOpen@8(const char* name, unsigned int flags);
stdcall int _BinkSetSoundSystem@8(SndOpenCallback open, unsigned int param);

stdcall IDirect3D8* Direct3DCreate8(UINT SDKVersion);
"#;

static WELL_KNOWN_CONVENTIONS: LazyLock<HashMap<String, CallingConvention>> = LazyLock::new(|| {
    let mut res = HashMap::with_capacity(ALL_DEFS.lines().count());
    for line in ALL_DEFS.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("#") {
            continue;
        }
        if let Some(def) = line.strip_prefix("stdcall ") {
            let (name, conv) = stdcall(def);
            let old = res.insert(name, conv);
            assert!(old.is_none(), "duplication of {:?}", def);
        } else if let Some(def) = line.strip_prefix("cdecl") {
            let (name, conv) = cdecl(def);
            let old = res.insert(name, conv);
            assert!(old.is_none(), "duplication of {:?}", def);
        } else if let Some(name) = line.strip_prefix("symbol") {
            res.insert(
                name.trim().trim_matches(';').to_string(),
                CallingConvention {
                    inputs: HashSet::new(),
                    outputs: HashSet::new(),
                    stack_adjust: 0,
                },
            );
        } else {
            panic!("unknown line: {line:?}")
        }
    }
    res
});

fn calling_convention_for(lib: &str, func: &str) -> CallingConvention {
    match WELL_KNOWN_CONVENTIONS.get(func) {
        Some(x) => x.clone(),
        None => panic!("unknown import: {lib}:{func}"),
    }
}

fn stdcall(def: &str) -> (String, CallingConvention) {
    let inputs = HashSet::from_iter([Io::Mem, Io::Esp]);
    let mut outputs = inputs.clone();

    let parsed = c_parse::parse_c_function_decl(def).unwrap();
    let stack_adjust = compute_stack_stdcall(&parsed);

    if parsed.return_type != "VOID" {
        outputs.insert(Io::Eax);
    }

    (
        parsed.name,
        CallingConvention {
            inputs,
            outputs,
            stack_adjust,
        },
    )
}

fn cdecl(def: &str) -> (String, CallingConvention) {
    let mut inputs = HashSet::from_iter([Io::Mem, Io::Esp]);
    let mut outputs = HashSet::from_iter([Io::Mem]);

    let parsed = match c_parse::parse_c_function_decl(def) {
        Ok(ok) => ok,
        Err(err) => panic!("failed to parse {def:?}: {err}"),
    };

    match parsed.return_type.as_str() {
        "void" | "VOID" => {}
        "double" | "voidfpu" => {
            outputs.insert(Io::X87Stack);
        }
        "intfpu" => {
            outputs.insert(Io::X87Stack);
            outputs.insert(Io::Eax);
        }
        _ => {
            outputs.insert(Io::Eax);
        }
    }

    if parsed
        .params
        .iter()
        .any(|x| matches!(x.ty.as_str(), "voidfpu"))
    {
        inputs.insert(Io::X87Stack);
    }

    (
        parsed.name,
        CallingConvention {
            inputs,
            outputs,
            stack_adjust: 0,
        },
    )
}

fn compute_stack_stdcall(parsed: &c_parse::FunctionDecl) -> u16 {
    let mut sum = 0;
    for arg in &parsed.params {
        let size = match arg.ty.as_str() {
            x if x.ends_with('*') => 4,
            "LONG"
            | "HKEY"
            | "LPCSTR"
            | "LPDWORD"
            | "LPBYTE"
            | "REGSAM"
            | "HINSTANCE"
            | "LPVOID"
            | "LPUNKNOWN"
            | "DWORD"
            | "PHKEY"
            | "REFIID"
            | "HDC"
            | "float"
            | "int"
            | "unsigned int"
            | "UINT"
            | "LPSIZE"
            | "HGDIOBJ"
            | "COLORREF"
            | "HANDLE"
            | "PLONG"
            | "LPWIN32_FIND_DATAA"
            | "PLARGE_INTEGER"
            | "LPFILETIME"
            | "LPOVERLAPPED"
            | "LPOVERLAPPED_COMPLETION_ROUTINE"
            | "LPCVOID"
            | "SIZE_T"
            | "LPMEMORYSTATUS"
            | "WINBOOL"
            | "LPOSVERSIONINFOA"
            | "LPCRITICAL_SECTION"
            | "LPCWSTR"
            | "HMODULE"
            | "LPSTR"
            | "LPSTARTUPINFOA"
            | "LPCWCH"
            | "LPCCH"
            | "LPBOOL"
            | "LPLONG"
            | "LPACCEL"
            | "LPRECT"
            | "HWND"
            | "HACCEL"
            | "WPARAM"
            | "LPARAM"
            | "HMONITOR"
            | "LPMONITORINFO"
            | "LPPOINT"
            | "PVOID"
            | "LPMSG"
            | "HMENU"
            | "HBINK"
            | "LPDIRECTSOUND"
            | "SndOpenCallback"
            | "LPSECURITY_ATTRIBUTES" => 4,
            "VOID" => 0,
            x => panic!("unknown type: {}", x),
        };
        sum += size
    }
    sum
}
