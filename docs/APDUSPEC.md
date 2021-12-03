# Tezos App

## General structure

The general structure of commands and responses is as follows:

#### Commands

| Field   | Type     | Content                | Note |
| :------ | :------- | :--------------------- | ---- |
| CLA     | byte (1) | Application Identifier | 0x80 |
| INS     | byte (1) | Instruction ID         |      |
| P1      | byte (1) | Parameter 1            |      |
| P2      | byte (1) | Parameter 2            |      |
| L       | byte (1) | Bytes in payload       |      |
| PAYLOAD | byte (L) | Payload                |      |

#### Response

| Field   | Type     | Content     | Note                     |
| ------- | -------- | ----------- | ------------------------ |
| ANSWER  | byte (?) | Answer      | depends on the command   |
| SW1-SW2 | byte (2) | Return code | see list of return codes |

#### Return codes

| Return code | Description             |
| ----------- | ----------------------- |
| 0x6400      | Execution Error         |
| 0x6982      | Empty buffer            |
| 0x6983      | Output buffer too small |
| 0x6986      | Command not allowed     |
| 0x6D00      | INS not supported       |
| 0x6E00      | CLA not supported       |
| 0x6F00      | Unknown                 |
| 0x9000      | Success                 |

---

## Command definition

### GetVersion

#### Command

| Field | Type     | Content                | Expected |
| ----- | -------- | ---------------------- | -------- |
| CLA   | byte (1) | Application Identifier | 0x80     |
| INS   | byte (1) | Instruction ID         | 0x10     |
| P1    | byte (1) | Parameter 1            | ignored  |
| P2    | byte (1) | Parameter 2            | ignored  |
| L     | byte (1) | Bytes in payload       | 0        |

#### Response

| Field     | Type     | Content          | Note                            |
| --------- | -------- | ---------------- | ------------------------------- |
| TEST      | byte (1) | Test Mode        | 0xFF means test mode is enabled |
| MAJOR     | byte (1) | Version Major    |                                 |
| MINOR     | byte (1) | Version Minor    |                                 |
| PATCH     | byte (1) | Version Patch    |                                 |
| LOCKED    | byte (1) | Device is locked |                                 |
| TARGET ID | byte (4) | Target ID        |                                 |
| SW1-SW2   | byte (2) | Return code      | see list of return codes        |

### INS_GET_ADDR

#### Command

| Field   | Type     | Content                   | Expected          |
| ------- | -------- | ------------------------- | ----------------- |
| CLA     | byte (1) | Application Identifier    | 0x80              |
| INS     | byte (1) | Instruction ID            | 0x11              |
| P1      | byte (1) | Request User confirmation | No = 0            |
| P2      | byte (1) | Curve identifier          | 0 = Ed25519       |
|         |          |                           | 1 = Secp256K1     |
|         |          |                           | 2 = Secp256R1     |
|         |          |                           | 3 = Ed25519 BIP32 |
| L       | byte (1) | Bytes in payload          | (depends)         |
| PathN   | byte (1) | Number of path components | ? (typically 4)   |
| Path[0] | byte (4) | Derivation Path Data      | 0x8000002c        |
| Path[1] | byte (4) | Derivation Path Data      | 0x800006c1        |
| Path[2] | byte (4) | Derivation Path Data      | ?                 |
| Path[3] | byte (4) | Derivation Path Data      | ?                 |
| Path[4] | byte (4) | Derivation Path Data      | ?                 |

#### Response

| Field      | Type      | Content           | Note                     |
| ---------- | --------- | ----------------- | ------------------------ |
| PK_LEN     | byte (1)  | Bytes in PKEY     |                          |
| PKEY       | byte (??) | Public key bytes  |                          |
| ADDR_HUMAN | byte (??) | Address as String | encoded with base58      |
| SW1-SW2    | byte (2)  | Return code       | see list of return codes |

### INS_SIGN

#### Command

| Field | Type     | Content                | Expected          |
| ----- | -------- | ---------------------- | ----------------- |
| CLA   | byte (1) | Application Identifier | 0x80              |
| INS   | byte (1) | Instruction ID         | 0x12              |
| P1    | byte (1) | Payload desc           | 0 = init          |
|       |          |                        | 1 = add           |
|       |          |                        | 2 = last          |
| P2    | byte (1) | Curve identifier       | 0 = Ed25519       |
|       |          |                        | 1 = Secp256K1     |
|       |          |                        | 2 = Secp256R1     |
|       |          |                        | 3 = Ed25519 BIP32 |
| L     | byte (1) | Bytes in payload       | (depends)         |

The first packet/chunk includes only the derivation path

All other packets/chunks contain data chunks that are described below

_First Packet_

| Field   | Type     | Content                   | Expected        |
|---------|----------|---------------------------|-----------------|
| PathN   | byte (1) | Number of path components | ? (typically 4) |
| Path[0] | byte (4) | Derivation Path Data      | 0x8000002c      |
| Path[1] | byte (4) | Derivation Path Data      | 0x800006c1      |
| Path[2] | byte (4) | Derivation Path Data      | ?               |
| Path[3] | byte (4) | Derivation Path Data      | ?               |
| Path[4] | byte (4) | Derivation Path Data      | ?               |

_Other Chunks/Packets_

| Field | Type     | Content | Expected |
| ----- | -------- | ------- | -------- |
| Data  | bytes... | Message |          |

Data is defined as:

| Field   | Type    | Content      | Expected |
| ------- | ------- | ------------ | -------- |
| Message | bytes.. | Data to sign |          |

#### Response

| Field    | Type            | Content     | Note                                  |
|----------|-----------------|-------------|---------------------------------------|
| SIG_HASH | byte (32)       | Signed hash | Blake2 hash used as signature message |
| SIG      | byte (variable) | Signature   | signature                             |
| SW1-SW2  | byte (2)        | Return code | see list of return codes              |

### INS_AUTHORIZE_BAKING

#### Command

| Field   | Type     | Content                   | Expected           |
|---------|----------|---------------------------|--------------------|
| CLA     | byte (1) | Application Identifier    | 0x80               |
| INS     | byte (1) | Instruction ID            | 0xA1               |
| P1      | byte (1) | Request User confirmation | Yes = 1, mandatory |
| P2      | byte (1) | Curve identifier          | 0 = Ed25519        |
|         |          |                           | 1 = Secp256K1      |
|         |          |                           | 2 = Secp256R1      |
|         |          |                           | 3 = Ed25519 BIP32  |
| L       | byte (1) | Bytes in payload          | (depends)          |
| PathN   | byte (1) | Number of path components | ? (typically 4)    |
| Path[0] | byte (4) | Derivation Path Data      | 0x8000002c         |
| Path[1] | byte (4) | Derivation Path Data      | 0x800006c1         |
| Path[2] | byte (4) | Derivation Path Data      | ?                  |
| Path[3] | byte (4) | Derivation Path Data      | ?                  |
| Path[4] | byte (4) | Derivation Path Data      | ?                  |

#### Response

| Field      | Type      | Content           | Note                     |
| ---------- | --------- | ----------------- | ------------------------ |
| PK_LEN     | byte (1)  | Bytes in PKEY     |                          |
| PKEY       | byte (??) | Public key bytes  |                          |
| SW1-SW2    | byte (2)  | Return code       | see list of return codes |

### INS_DEAUTHORIZE_BAKING

#### Command

| Field   | Type     | Content                   | Expected           |
|---------|----------|---------------------------|--------------------|
| CLA     | byte (1) | Application Identifier    | 0x80               |
| INS     | byte (1) | Instruction ID            | 0xAC               |
| P1      | byte (1) | Request User confirmation | Yes = 1, mandatory |
| P2      | byte (1) | ignored                   |                    |
| L       | byte (1) | Bytes in payload          | 0                  |

#### Response

| Field      | Type      | Content           | Note                     |
| ---------- | --------- | ----------------- | ------------------------ |
| SW1-SW2    | byte (2)  | Return code       | see list of return codes |

### INS_QUERY_AUTH_KEY

#### Command

| Field | Type     | Content                   | Expected |
|-------|----------|---------------------------|----------|
| CLA   | byte (1) | Application Identifier    | 0x80     |
| INS   | byte (1) | Instruction ID            | 0xA7     |
| P1    | byte (1) | Request User confirmation | No = 0   |
| P2    | byte (1) | ignored                   |          |
| L     | byte (1) | Bytes in payload          | 0        |

#### Response

| Field      | Type      | Content           | Note                     |
| ---------- | --------- | ----------------- | ------------------------ |
| PathN   | byte (1) | Number of path components | ? (typically 4)          |
| Path[0] | byte (4) | Derivation Path Data      | 0x8000002c               |
| Path[1] | byte (4) | Derivation Path Data      | 0x800006c1               |
| Path[2] | byte (4) | Derivation Path Data      | ?                        |
| Path[3] | byte (4) | Derivation Path Data      | ?                        |
| Path[4] | byte (4) | Derivation Path Data      | ?                        |
| SW1-SW2    | byte (2)  | Return code       | see list of return codes |

### INS_QUERY_AUTH_KEY_WITH_CURVE

#### Command

| Field | Type     | Content                   | Expected |
|-------|----------|---------------------------|----------|
| CLA   | byte (1) | Application Identifier    | 0x80     |
| INS   | byte (1) | Instruction ID            | 0xAD     |
| P1    | byte (1) | Request User confirmation | No = 0   |
| P2    | byte (1) | ignored                   |          |
| L     | byte (1) | Bytes in payload          | 0        |

#### Response

| Field   | Type     | Content                   | Note                     |
|---------|----------|---------------------------|--------------------------|
| Curve   | byte (1) | 0 = Ed25519               |                          |
|         |          | 1 = Secp256K1             |                          |
|         |          | 2 = Secp256R1             |                          |
|         |          | 3 = Ed25519 BIP32         |                          |
| PathN   | byte (1) | Number of path components | ? (typically 4)          |
| Path[0] | byte (4) | Derivation Path Data      | 0x8000002c               |
| Path[1] | byte (4) | Derivation Path Data      | 0x800006c1               |
| Path[2] | byte (4) | Derivation Path Data      | ?                        |
| Path[3] | byte (4) | Derivation Path Data      | ?                        |
| Path[4] | byte (4) | Derivation Path Data      | ?                        |
| SW1-SW2 | byte (2) | Return code               | see list of return codes |

### INS_BAKER_SIGN

Same as `INS_SIGN`, except the `INS` field is `0xAF`.

The difference lies in the interpretation of the message from the other chunks/packets.

# Legacy app

CLA is 0x80

https://github.com/obsidiansystems/ledger-app-tezos/blob/58797b2f9606c5a30dd1ccc9e5b9962e45e10356/src/main.c#L10-L36
https://github.com/obsidiansystems/ledger-app-tezos

// Instruction codes

| Command                       | INS  |                                 |
|-------------------------------|------|---------------------------------|
| INS_VERSION                   | 0x00 |                                 |
| INS_AUTHORIZE_BAKING          | 0x01 |                                 |
| INS_GET_PUBLIC_KEY            | 0x02 |                                 |
| INS_PROMPT_PUBLIC_KEY         | 0x03 |                                 |
| INS_SIGN                      | 0x04 |                                 |
| INS_SIGN_UNSAFE               | 0x05 | // Data that is already hashed. |
| INS_RESET                     | 0x06 |                                 |
| INS_QUERY_AUTH_KEY            | 0x07 |                                 |
| INS_QUERY_MAIN_HWM            | 0x08 |                                 |
| INS_GIT                       | 0x09 |                                 |
| INS_SETUP                     | 0x0A |                                 |
| INS_QUERY_ALL_HWM             | 0x0B |                                 |
| INS_DEAUTHORIZE               | 0x0C |                                 |
| INS_QUERY_AUTH_KEY_WITH_CURVE | 0x0D |                                 |
| INS_HMAC                      | 0x0E |                                 |
| INS_SIGN_WITH_HASH            | 0x0F |                                 |
