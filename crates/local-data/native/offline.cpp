// Navzeroth's fixed, C-ABI wrapper around pinned CascLib. BSD-3-Clause.
#include "CascLib.h"
#include "CascCommon.h"
#include <stdint.h>
#include <string.h>
#include <new>

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
// Fail-closed check of the never-decrypt build patch (build.rs): after the
// built-in keys would have loaded, the key map does not hold a key name that
// CascLib's static table carries; and even with that name added to the map
// (zero bytes, no key material here), an encrypted frame naming it is reported
// as encrypted with an unavailable key and nothing is written to the output.
// Returns 0 only when decryption is disabled.
uint32_t nav_casc_decryption_disabled(void) {
    const ULONGLONG name = 0x2C547F26A2613E01ULL; // First StaticCascKeys entry.
    TCascStorage * hs = new(std::nothrow) TCascStorage();
    if(hs == NULL) return 16;
    uint32_t result = 0;
    if(CascLoadEncryptionKeys(hs) != ERROR_SUCCESS) result |= 1;
    if(hs->KeyMap.FindKey(name) != NULL) result |= 2;
    BYTE zero[CASC_KEY_LENGTH] = {};
    if(!hs->KeyMap.AddKey(name, zero)) result |= 16;
    // BLTE 'E' frame body (mode byte stripped): key-name size, key name, IV size,
    // IV, cipher 'S' (Salsa20), then the encrypted payload.
    BYTE frame[1 + 8 + 1 + 4 + 1 + 16] = {};
    frame[0] = 8;
    memcpy(frame + 1, &name, 8);
    frame[9] = 4;
    frame[14] = 'S';
    BYTE out[16];
    memset(out, 0xEE, sizeof(out));
    DWORD length = sizeof(out);
    if(CascDecrypt(hs, out, &length, frame, sizeof(frame), 0) != ERROR_FILE_ENCRYPTED || hs->LastFailKeyName != name) result |= 4;
    for(size_t i = 0; i < sizeof(out); i++) if(out[i] != 0xEE) result |= 8;
    hs->Release();
    return result;
}
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
    if(nav_casc_decryption_disabled() != 0) return ERROR_NOT_SUPPORTED;
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
// Frame-aware partial read (V2.2d). Frame tables are loaded through the public
// span-info query; each frame's layout comes from CascLib's own frame records.
static PCASC_FILE_FRAME frame_at(TCascFile * hf, uint32_t index, DWORD * flags, TFileStream ** stream) {
    for(DWORD span = 0; span < hf->SpanCount; span++) {
        PCASC_FILE_SPAN pSpan = hf->pFileSpan + span;
        if(pSpan->pFrames == NULL) return NULL;
        if(index < pSpan->FrameCount) {
            *flags = hf->pCKeyEntry[span].Flags;
            *stream = pSpan->pStream;
            return pSpan->pFrames + index;
        }
        index -= pSpan->FrameCount;
    }
    return NULL;
}
uint32_t nav_casc_frame_count(void * file, uint32_t * count) {
    *count = 0;
    TCascFile * hf = TCascFile::IsValid(file);
    if(hf == NULL || hf->SpanCount == 0 || hf->SpanCount > 256) return 6;
    static CASC_FILE_SPAN_INFO spans[256]; // Serialized native access; scratch only.
    if(!CascGetFileInfo(file, CascFileSpanInfo, spans, sizeof(spans), NULL)) return read_status(GetCascError());
    ULONGLONG total = 0;
    for(DWORD span = 0; span < hf->SpanCount; span++) {
        if(hf->pFileSpan[span].pFrames == NULL) return 6;
        total += hf->pFileSpan[span].FrameCount;
    }
    if(total > 0xFFFFFFFFull) return 5;
    *count = (uint32_t)total;
    return 0;
}
// One frame's logical range and whether it is BLTE mode 'E'. The frame's encoded
// bytes are first MD5-checked against the frame table (strict). An 'E' frame is
// only identified, never decoded or decrypted; its bytes are released here.
uint32_t nav_casc_frame(void * file, uint32_t index, uint64_t * start, uint32_t * length, uint32_t * encrypted) {
    *start = 0; *length = 0; *encrypted = 0;
    TCascFile * hf = TCascFile::IsValid(file);
    if(hf == NULL) return 6;
    DWORD flags = 0; TFileStream * stream = NULL;
    PCASC_FILE_FRAME frame = frame_at(hf, index, &flags, &stream);
    if(frame == NULL || frame->EndOffset < frame->StartOffset || frame->EndOffset - frame->StartOffset != frame->ContentSize) return 6;
    *start = frame->StartOffset;
    *length = frame->ContentSize;
    // Plain and raw-ZLIB spans are not BLTE-framed and cannot be encrypted.
    if(flags & (CASC_CE_PLAIN_DATA | CASC_CE_ZLIB_DATA)) return 0;
    if(stream == NULL || frame->EncodedSize == 0) return 4;
    if(frame->EncodedSize > 64u * 1024u * 1024u) return 5;
    LPBYTE encoded = CASC_ALLOC<BYTE>(frame->EncodedSize);
    if(encoded == NULL) return 5;
    ULONGLONG offset = frame->DataFileOffset;
    uint32_t status = 0;
    if(!FileStream_Read(stream, &offset, encoded, frame->EncodedSize)) status = 6;
    else if(hf->bVerifyIntegrity && !CascVerifyDataBlockHash(encoded, frame->EncodedSize, frame->FrameHash.Value)) status = 4;
    else *encrypted = encoded[0] == 'E' ? 1 : 0;
    CASC_FREE(encoded);
    return status;
}
// Strict read of [offset, offset + length). The caller passes only runs of frames
// identified as not encrypted. Same status contract as nav_casc_read.
uint32_t nav_casc_read_at(void * file, uint64_t offset, void * buffer, uint32_t length, uint32_t * done) {
    *done = 0;
    if(offset > 0x7FFFFFFFFFFFFFFFull || !CascSetFilePointer64(file, (LONGLONG)offset, NULL, FILE_BEGIN)) return 6;
    return nav_casc_read(file, buffer, length, done);
}
void nav_casc_file_close(void * file) { CascCloseFile(file); }
}
