// Navzeroth's fixed, C-ABI wrapper around pinned CascLib. BSD-3-Clause.
#include "CascLib.h"
#include "CascCommon.h"
#include <stdint.h>

// The out-of-scope Overwatch reader and its crypto sources are not linked.
DWORD RootHandler_CreateOverwatch(TCascStorage *, CASC_BLOB &) { return ERROR_NOT_SUPPORTED; }

static bool WINAPI reject_download(void *, CASC_PROGRESS_MSG message, LPCSTR, DWORD, DWORD) {
    return message == CascProgressDownloadingFile || message == CascProgressDownloadingArchiveIndexes;
}
// Stable private ABI: OS-specific errno/Win32 values never cross the boundary.
static uint32_t read_status(DWORD error) {
    if(error == ERROR_FILE_NOT_FOUND || error == ERROR_PATH_NOT_FOUND) return 1;
    if(error == ERROR_FILE_ENCRYPTED) return 2;
    if(error == ERROR_NOT_SUPPORTED || error == ERROR_BAD_FORMAT) return 3;
    if(error == ERROR_FILE_CORRUPT) return 4;
    if(error == ERROR_NOT_ENOUGH_MEMORY) return 5;
    // Reached from the open stage (BLTE header reads); read-stage truncation is
    // zero-filled by the local stream and fails the strict frame MD5 instead.
    if(error == ERROR_HANDLE_EOF || error == ERROR_FILE_INCOMPLETE) return 7;
    return 6; // Inaccessible/unrecognized failure, not an assertion of corruption.
}

extern "C" {
uint32_t nav_casc_open(const TCHAR * path, const TCHAR * product, uint32_t locale, uint32_t expected_build, void ** result) {
    CASC_OPEN_STORAGE_ARGS args = {};
    args.Size = sizeof(args);
    args.szLocalPath = path;
    args.szCodeName = product;
    // Zero selects the explicitly selected product's installed default locale.
    // Opening ALL here selects the first root variant, which may not be installed.
    args.dwLocaleMask = locale;
    args.dwFlags = 0; // No online, allow-download or force-download.
    args.PfnProgressCallback = reject_download;
    *result = NULL;
    if(!CascOpenStorageEx(NULL, &args, false, result)) return GetCascError();
    CASC_STORAGE_PRODUCT actual = {};
    if(!CascGetStorageInfo(*result, CascStorageProduct, &actual, sizeof(actual), NULL) || actual.BuildNumber != expected_build || strcmp(actual.szCodeName,"wow_classic_beta") != 0) {
        CascCloseStorage(*result); *result=NULL; return ERROR_BAD_FORMAT;
    }
    return 0;
}
void nav_casc_close(void * storage) { CascCloseStorage(storage); }
uint32_t nav_casc_stats(void * storage, uint32_t * features, uint32_t * local, uint32_t * total) {
    if(!CascGetStorageInfo(storage, CascStorageFeatures, features, sizeof(*features), NULL) ||
       !CascGetStorageInfo(storage, CascStorageLocalFileCount, local, sizeof(*local), NULL) ||
       !CascGetStorageInfo(storage, CascStorageTotalFileCount, total, sizeof(*total), NULL)) return GetCascError();
    return 0;
}
uint32_t nav_casc_file_open(void * storage, uint32_t id, void ** result, uint64_t * size) {
    *result = NULL;
    if(!CascOpenFile(storage, CASC_FILE_DATA_ID(id), CASC_LOCALE_ALL_WOW, CASC_OPEN_BY_FILEID | CASC_STRICT_DATA_CHECK, result))
        return read_status(GetCascError());
    ULONGLONG length = 0;
    if(!CascGetFileSize64(*result, &length)) {
        DWORD error = GetCascError(); CascCloseFile(*result); *result = NULL; return read_status(error);
    }
    *size = length;
    return 0;
}
uint32_t nav_casc_name_open(void * storage, const char * name, void ** result, uint64_t * size) {
    *result = NULL;
    if(!CascOpenFile(storage, name, CASC_LOCALE_ALL_WOW, CASC_OPEN_BY_NAME | CASC_STRICT_DATA_CHECK, result)) return read_status(GetCascError());
    ULONGLONG length = 0;
    if(!CascGetFileSize64(*result, &length)) { DWORD error=GetCascError(); CascCloseFile(*result); *result=NULL; return read_status(error); }
    *size=length; return 0;
}
// CascReadFile reports success with a short count when a later BLTE frame fails
// (an encrypted frame with an unknown key, a frame checksum mismatch): the
// frame's error is left in the last-error slot. Report that cause instead of a
// generic corruption; a short count with no recorded error stays its own code (8).
uint32_t nav_casc_read(void * file, void * buffer, uint32_t length, uint32_t * done) {
    DWORD read = 0;
    *done = 0;
    SetCascError(ERROR_SUCCESS);
    if(!CascReadFile(file, buffer, length, &read)) return read_status(GetCascError());
    *done = read;
    if(read == length) return 0;
    DWORD error = GetCascError();
    return error == ERROR_SUCCESS ? 8 : read_status(error);
}
void nav_casc_file_close(void * file) { CascCloseFile(file); }
}
