#ifndef PALE_DARWIN_SIGNING_V1_H
#define PALE_DARWIN_SIGNING_V1_H
#include <stdint.h>

/* Private native ABI: fixed byte buffers, no pointers or secret-key arguments.
 * Rust owns the closed JSON request and response status definitions. */
typedef struct {
    uint32_t length;
    uint8_t bytes[4096];
} PaleSigningRequestV1;
typedef struct {
    uint32_t status;
    uint8_t signature[64];
} PaleSigningResponseV1;
PaleSigningResponseV1 pale_darwin_signing_execute_v1(PaleSigningRequestV1 request);
uint32_t pale_darwin_signing_is_fixture_v1(void);
typedef struct {
    uint32_t status;
    uint32_t length;
    uint8_t block0[4096];
    uint8_t block1[4096];
    uint8_t block2[4096];
    uint8_t block3[4096];
} PaleSigningPolicyCaptureV1;
PaleSigningPolicyCaptureV1 pale_darwin_signing_capture_policy_v1(PaleSigningRequestV1 path);
#endif
