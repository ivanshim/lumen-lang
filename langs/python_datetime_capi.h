/* Native datetime interface, capsule name "lumen.datetime_CAPI.v1".
 * This is an opaque-handle lumen ABI, not CPython's PyObject/PyDateTime_CAPI ABI.
 * Obtain the capsule pointer through the hosting kernel's capsule API.
 * The table and its seven object handles are borrowed, read-only and valid
 * while the capsule lives. retain() produces an independently owned handle.
 * All handles and contexts must belong to the same kernel and interpreter.
 * A context is borrowed for one synchronous callback on the interpreter thread;
 * the embedding host supplies it with with_context()/lend_machine().
 * Constructors return owned objects, or NULL and an owned error in *error.
 * A non-NULL error output pointer is required and is cleared on success.
 * The host may resume the original Python exception using take_error()/own_error(),
 * or dispose of it with release_error(). Objects are disposed with release().
 * Objects and errors can outlive the table, but not their owning interpreter.
 * NULL tzinfo means None; NULL timezone name omits the optional name.
 * from_timestamp accepts a tuple handle and an optional dictionary handle.
 * Attribute names are NUL-terminated UTF-8, borrowed for the callback only.
 */
#ifndef LUMEN_DATETIME_CAPI_H
#define LUMEN_DATETIME_CAPI_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef void *LumenDateTimeObject;
typedef void *LumenDateTimeContext;
typedef void *LumenDateTimeError;
typedef struct LumenDateTimeAPI {
    uint64_t version;
    size_t size;
    LumenDateTimeObject date_type, datetime_type, time_type, delta_type;
    LumenDateTimeObject tzinfo_type, timezone_type, utc;
    LumenDateTimeObject (*retain)(LumenDateTimeObject);
    void (*release)(LumenDateTimeObject);
    void (*release_error)(LumenDateTimeError);
    LumenDateTimeObject (*date)(LumenDateTimeContext, LumenDateTimeObject, int32_t, int32_t, int32_t, LumenDateTimeError *);
    LumenDateTimeObject (*datetime)(LumenDateTimeContext, LumenDateTimeObject, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, LumenDateTimeObject, int32_t, LumenDateTimeError *);
    LumenDateTimeObject (*time)(LumenDateTimeContext, LumenDateTimeObject, int32_t, int32_t, int32_t, int32_t, LumenDateTimeObject, int32_t, LumenDateTimeError *);
    LumenDateTimeObject (*delta)(LumenDateTimeContext, LumenDateTimeObject, int32_t, int32_t, int32_t, LumenDateTimeError *);
    LumenDateTimeObject (*timezone)(LumenDateTimeContext, LumenDateTimeObject, LumenDateTimeObject, LumenDateTimeObject, LumenDateTimeError *);
    LumenDateTimeObject (*from_timestamp)(LumenDateTimeContext, LumenDateTimeObject, LumenDateTimeObject, LumenDateTimeObject, LumenDateTimeError *);
    LumenDateTimeObject (*attribute)(LumenDateTimeContext, LumenDateTimeObject, const char *, LumenDateTimeError *);
} LumenDateTimeAPI;
#ifdef __cplusplus
}
#endif
#endif
