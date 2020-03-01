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

---------

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

| Field   | Type     | Content          | Note                            |
| ------- | -------- | ---------------- | ------------------------------- |
| TEST    | byte (1) | Test Mode        | 0xFF means test mode is enabled |
| MAJOR   | byte (1) | Version Major    |                                 |
| MINOR   | byte (1) | Version Minor    |                                 |
| PATCH   | byte (1) | Version Patch    |                                 |
| LOCKED  | byte (1) | Device is locked |                                 |
| SW1-SW2 | byte (2) | Return code      | see list of return codes        |

### INS_GET_ADDR

#### Command

| Field   | Type     | Content                   | Expected   |
| ------- | -------- | ------------------------- | ---------- |
| CLA     | byte (1) | Application Identifier    | 0x80       |
| INS     | byte (1) | Instruction ID            | 0x11       |
| P1      | byte (1) | Request User confirmation | No = 0     |
| P2      | byte (1) | Parameter 2               | ignored    |
| L       | byte (1) | Bytes in payload          | (depends)  |
| Path[0] | byte (4) | Derivation Path Data      | 0x8000002c |
| Path[1] | byte (4) | Derivation Path Data      | 0x800006c1 |
| Path[2] | byte (4) | Derivation Path Data      | ?          |
| Path[3] | byte (4) | Derivation Path Data      | ?          |
| Path[4] | byte (4) | Derivation Path Data      | ?          |

#### Response

| Field          | Type      | Content              | Note                     |
| -------------- | --------- | -------------------- | ------------------------ |
| PK             | byte (65) | Public Key           |                          |
| ADDR_RAW_LEN   | byte (1)  | ADDR_RAW Length      |                          |
| ADDR_RAW       | byte (??) | Address as Raw Bytes |                          |
| ADDR_HUMAN_LEN | byte (1)  | ADDR_HUMAN Len       |                          |
| ADDR_HUMAN     | byte (??) | Address as String    |                          |
| SW1-SW2        | byte (2)  | Return code          | see list of return codes |

---

### INS_SIGN

#### Command

| Field | Type     | Content                | Expected  |
| ----- | -------- | ---------------------- | --------- |
| CLA   | byte (1) | Application Identifier | 0x80      |
| INS   | byte (1) | Instruction ID         | 0x12      |
| P1    | byte (1) | Payload desc           | 0 = init  |
|       |          |                        | 1 = add   |
|       |          |                        | 2 = last  |
| P2    | byte (1) | ----                   | not used  |
| L     | byte (1) | Bytes in payload       | (depends) |

The first packet/chunk includes only the derivation path

All other packets/chunks contain data chunks that are described below

_First Packet_

| Field   | Type     | Content              | Expected   |
| ------- | -------- | -------------------- | ---------- |
| Path[0] | byte (4) | Derivation Path Data | 0x8000002c |
| Path[1] | byte (4) | Derivation Path Data | 0x800006c1 |
| Path[2] | byte (4) | Derivation Path Data | ?          |
| Path[3] | byte (4) | Derivation Path Data | ?          |
| Path[4] | byte (4) | Derivation Path Data | ?          |

*Other Chunks/Packets*

| Field | Type     | Content | Expected |
| ----- | -------- | ------- | -------- |
| Data  | bytes... | Message |          |

Data is defined as:

| Field   | Type    | Content      | Expected |
| ------- | ------- | ------------ | -------- |
| Message | bytes.. | Data to sign |          |

#### Response

| Field       | Type            | Content     | Note                     |
| ----------- | --------------- | ----------- | ------------------------ |
| SIG         | byte (variable) | Signature   | signature                |
| SW1-SW2     | byte (2)        | Return code | see list of return codes |

# Legacy app

CLA is 0x80

https://github.com/obsidiansystems/ledger-app-tezos/blob/58797b2f9606c5a30dd1ccc9e5b9962e45e10356/src/main.c#L10-L36
https://github.com/obsidiansystems/ledger-app-tezos

// Instruction codes

| Command | INS |
| --- | --- |
| INS_VERSION |  0x00 |
| INS_AUTHORIZE_BAKING |  0x01 |
| INS_GET_PUBLIC_KEY |  0x02 |
| INS_PROMPT_PUBLIC_KEY |  0x03 |
| INS_SIGN |  0x04 |
| INS_SIGN_UNSAFE |  0x05 |  // Data that is already hashed.
| INS_RESET |  0x06 |
| INS_QUERY_AUTH_KEY |  0x07 |
| INS_QUERY_MAIN_HWM |  0x08 |
| INS_GIT |  0x09 |
| INS_SETUP |  0x0A |
| INS_QUERY_ALL_HWM |  0x0B |
| INS_DEAUTHORIZE |  0x0C |
| INS_QUERY_AUTH_KEY_WITH_CURVE |  0x0D |
| INS_HMAC |  0x0E |
| INS_SIGN_WITH_HASH |  0x0F |
