# Notes on Tezos protocol

This document is a collection of notes on the tezos protocol and related concepts useful for this project

# Encoding scheme

This section describes the encoding schemes used in tezos

On most information provided there will be the command(s) to the `tezos-codec` utility
where the information was retrieved from

## Operations

An encoded tezos operation is represented as follows:

`tezos-codec describe alpha.operation binary schema`

| Name      | Size | Contents                 |
| --------- | ---- | ------------------------ |
| branch    | 32   | [Bytes]                  |
| contents  |      | Sequence of [Operations] |
| signature | 64   | [Bytes] (optional)       |

Example (omitting contents):

| Name       | Hex                                                                                                                              | Value                                                                                            |
| :--------- | :------------------------------------------------------------------------------------------------------------------------------- | :----------------------------------------------------------------------------------------------- |
| branch     | 008f1d96e2783258ff663f03dacfe946c026a5d194c73d1987b3da73fadea7d4                                                                 | BKiXcfN1ZTXnNNbTWSRArSWzVFc6om7radWq5mTqGX6rY4P2Uhe                                              |
| contents[] | ...                                                                                                                              | ...                                                                                              |
| signature  | 9595facf847a72b4c3fe231c0e4185e68e9b2875aa3c639382c86bcf0af23699f47fe66a6550ade936a5b59d5919ad20703885750314e0c368b277de39e7d10a | sighZMqWz5G8drK1VTsmTnQBFEQ9kxQQxL88NFh8UaqDEJ3R3mzgR3g81azadZ9saPwsWga3kEPsyfbzrXm6ueuDvx3pQ5Q9 |

To retrieve the value of the branch from the hex:

1. Append `0x134` prefix
2. Base58 encode with checksum

To retrieve the value of the signature from the hex:

1. Append `0x4822b` prefix
2. Base58 encode with checksum

[source](https://tezos.stackexchange.com/questions/2907/how-are-tezos-operations-encoded) of the above instructions

### Operation types

There are many operation content types, each prefixed with a tag and then the contents follow it

| Tag  | Name                          |
| :--- | :---------------------------- |
| 0x00 | [Endorsement]                 |
| 0x01 | [Seed Nonce Revelation]       |
| 0x02 | [Double endorsement evidence] |
| 0x03 | [Double baking evidence]      |
| 0x04 | [Activate account]            |
| 0x05 | [Proposals]                   |
| 0x06 | [Ballot]                      |
| 0x0A | [Endorsement with slot]       |
| 0x11 | [Failing Noop]                |
| 0x6B | [Reveal]                      |
| 0x6C | [Transaction]                 |
| 0x6D | [Origination]                 |
| 0x6E | [Delegation]                  |

#### Endorsement

`tezos-codec describe alpha.operation.contents binary schema` (search `Endorsement` section)

An endorsement is encoded as follows:

| Name  | Size | Contents                   |
|:------|:-----|:---------------------------|
| tag   | 1    | 0x00                       |
| level | 4    | Signed 32-bit integer (BE) |

#### Seed nonce revelation

`tezos-codec describe alpha.operation.contents binary schema` (search `Seed_nonce_revelation` section)

An endorsement is encoded as follows:

| Name  | Size | Contents                   |
|:------|:-----|:---------------------------|
| tag   | 1    | 0x01                       |
| level | 4    | Signed 32-bit integer (BE) |
| nonce | 32   | [Bytes]                    |

#### Double endorsement evidence

`tezos-codec describe alpha.operation.contents binary schema` (search `Double_endorsement_evidence` section)

An double endorsement evidence is encoded as follows:

| Name        | Size | Contents                      |
|:------------|:-----|:------------------------------|
| tag         | 1    | 0x02                          |
| length\_op1 | 4    | length of the next field (BE) |
| op1         |      | [Inlined endorsement]         |
| length\_op2 | 4    | lenght of the next field (BE) |
| op2         |      | [Inlined endorsement]         |
| slot        | 2    | Unsigned 16-bit integer (BE)  |

#### Double baking evidence

`tezos-codec describe alpha.operation.contents binary schema` (search `Double_baking_evidence` section)

An double baking evidence is encoded as follows:

| Name        | Size | Contents                      |
|:------------|:-----|:------------------------------|
| tag         | 1    | 0x02                          |
| length\_bh1 | 4    | length of the next field (BE) |
| bh1         |      | [Inlined block header]        |
| length\_bh2 | 4    | lenght of the next field (BE) |
| bh2         |      | [Inlined block header]        |

##### Inlined block header

`tezos-codec describe alpha.operation.contents binary schema` (search `alpha.block_header.alpha.full_header` section)

An inlined block header is encoded as follows:

| Name                            | Size | Contents                     |
|:--------------------------------|:-----|:-----------------------------|
| level                           | 4    | Signed 32-bit integer (BE)   |
| proto                           | 1    | Unsigned 8-bit integer       |
| predecessor                     | 32   | [Bytes]                      |
| timestamp                       | 8    | Signed 64-bit integer (BE)   |
| validation\_pass                | 1    | Unsigned 8-bit integer       |
| operation\_hash                 | 32   | [Bytes]                      |
| fitness len                     | 4    | length of next field (BE)    |
| fitness                         |      | Sequence of [fitness]        |
| context                         | 32   | [Bytes]                      |
| priority                        | 2    | Unsigned 16-bit integer (BE) |
| proof\_of\_work\_nonce          | 8    | [Bytes]                      |
| seed\_nonce\_hash?              | 1    | [bool]                       |
| seed\_nonce\_hash               | 32   | [Bytes]                      |
| liquidity\_baking\_escape\_vote | 1    | [bool]                       |
| signature                       | 64   | [Bytes]                      |

##### Fitness

`tezos-codec describe alpha.operation.contents binary schema` (search `fitness.elem` section)

A fitness elem is encoded as follows:

| Name           | Size | Contents               |
|:---------------|:-----|:-----------------------|
| fitness length | 4    | Next field length (BE) |
| fitness        |      | Fitness [bytes]        |

#### Activate account

`tezos-codec describe alpha.operation.contents binary schema` (search `Activate_account` section)

An account activation is encoded as follows:

| Name   | Size | Contents          |
|:-------|:-----|:------------------|
| tag    | 1    | 0x04              |
| pkh    | 20   | [Public Key Hash] |
| secret | 20   | [Bytes]           |

The public key hash is not prefixed with a tag because it's always Ed25519

#### Proposals 

`tezos-codec describe alpha.operation.contents binary schema` (search `Proposals` section)

An endorsement is encoded as follows:

| Name             | Size | Contents                   |
|:-----------------|:-----|:---------------------------|
| tag              | 1    | 0x05                       |
| source           | 21   | [Public Key Hash]          |
| period           | 4    | Signed 32-bit integer (BE) |
| proposals length | 4    | Next field length (BE)     |
| proposals        |      | Proposals [bytes]          |

Proposals is a sequence of sequences of 32 bytes, each representing a different proposal

#### Ballot

`tezos-codec describe alpha.operation.contents binary schema` (search `Ballot` section)

An endorsement is encoded as follows:

| Name     | Size | Contents              |
|:---------|:-----|:----------------------|
| tag      | 1    | 0x06                  |
| source   | 21   | [Public Key Hash]     |
| period   | 4    | Signed 32-bit integer |
| proposal | 32   | [Bytes]               |
| ballot   | 1    | [Vote]                |

##### Vote

A ballot `vote` is encoded as follows:

| Value | Meaning           |
|-------|-------------------|
| 0x00  | yay               |
| 0x01  | nay               |
| 0x02  | pass              |


#### Endorsement with slot

`tezos-codec describe alpha.operation.contents binary schema` (search `Endorsement_with_slot` section)

An endorsement with slot is encoded as follows:

| Name        | Size | Contents                     |
|:------------|:-----|:-----------------------------|
| tag         | 1    | 0x10                         |
| length      | 4    | length of next field (BE)    |
| endorsement |      | [Inlined endorsement]        |
| slot        | 2    | Unsigned 16-bit integer (BE) |

The length field is the total length in bytes of the operation, excluding the tag and slot

##### Inlined endorsement

`tezos-codec describe alpha.operation.contents binary schema` (search `alpha.inlined.endorsement` section)

An inlined endorsement is encoded as follows:

| Name            | Size | Contents                         |
|:----------------|:-----|:---------------------------------|
| branch          | 32   | [Bytes]                          |
| endorsement_tag | 1    | 0x00                             |
| level           | 4    | Signed 32-bit integer            |
| signature       |      | [Bytes]                          |

An inlined endorsement is just a helper structure for endorsement with slots and double endorsement evidence

#### Failing Noop

`tezos-codec describe alpha.operation.contents binary schema` (search `Failing_noop` section)

A failing noop is encoded as follows:

| Name      | Size | Contents               |
|:----------|:-----|:-----------------------|
| tag       | 1    | 0x11                   |
| length    | 4    | next field length (BE) |
| arbitrary |      | [Bytes]                |


#### Reveal

`tezos-codec describe alpha.operation.contents binary schema` (search `Reveal` section)

A reveal is encoded as follows:

| Name           | Size | Contents          |
|:---------------|:-----|:------------------|
| tag            | 1    | 0x6B              |
| source         | 21   | [Public Key Hash] |
| fee            |      | [Zarith]          |
| counter        |      | [Zarith]          |
| gas\_limit     |      | [Zarith]          |
| storage\_limit |      | [Zarith]          |
| public\_key    |      | [Public Key]      |

#### Transaction

`tezos-codec describe alpha.operation.contents binary schema` (search `Transaction` section)

A transaction is encoded as follows:

| Name           | Size | Contents                 |
|:---------------|:-----|:-------------------------|
| tag            | 1    | 0x6C                     |
| source         | 21   | [Public Key Hash]        |
| fee            |      | [Zarith]                 |
| counter        |      | [Zarith]                 |
| gas\_limit     |      | [Zarith]                 |
| storage\_limit |      | [Zarith]                 |
| amount         |      | [Zarith]                 |
| destination    | 22   | [Contract ID]            |
| parameters?    | 1    | [bool]                   |
| parameters     |      | [Transaction parameters] |

##### Parameters

`tezos-codec describe alpha.operation.contents binary schema` (search `X_0`)

Transaction parameters are used for originated contracts to execute
the specified [entrypoint] with the passed parameters.

Parameters are encoded as follows:
| Name       | Size | Contents               |
|------------|------|------------------------|
| entrypoint |      | [Entrypoint]           |
| length     | 4    | next field length (BE) |
| parameters |      | Michelson [Bytes]      |

###### Entrypoint

`tezos-codec describe alpha.operation.contents binary schema` (search `alpha.entrypoint`)

Entrypoints are all prefixed with a tag that encodes a common named entrypoint or
a custom named entrypoint

| Tag  | Entrypoint        |
| ---- | ----------------- |
| 0x00 | "default"         |
| 0x01 | "root"            |
| 0x02 | "do"              |
| 0x03 | "set_delegate"    |
| 0x04 | "remove_delegate" |
| 0xFF | custom            |

Custom named entrypoints are encoded as follows:

| Name   | Size | Contents          |
| ------ | ---- | ----------------- |
| length | 1    | next field length |
| bytes  |      | UTF-8 [Bytes]     |

#### Origination

`tezos-codec describe alpha.operation.contents binary schema` (search `Origination` section)

A transaction is encoded as follows:

| Name           | Size | Contents          |
|:---------------|:-----|:------------------|
| tag            | 1    | 0x6D              |
| source         | 21   | [Public Key Hash] |
| fee            |      | [Zarith]          |
| counter        |      | [Zarith]          |
| gas\_limit     |      | [Zarith]          |
| storage\_limit |      | [Zarith]          |
| balance        |      | [Zarith]          |
| delegate?      | 1    | [bool]            |
| delegate       | 21   | [Public Key Hash] |
| script         |      | [Script]          |

##### Script

`tezos-codec describe alpha.operation.contents binary schema` (search `alpha.scripted.contracts` section)

Scripts in an [Origination] operation contain 2 major sections, `code` and `storage`,
these are both sequence of byets and are prepended with their length as follows:

| Name    | Size | Contents               |
|---------|------|------------------------|
| length  | 4    | next field length (BE) |
| code    |      | [Bytes]                |
| length  | 4    | next field length (BE) |
| storage |      | [Bytes]                |


#### Delegation

`tezos-codec describe alpha.operation.contents binary schema` (search `Delegation` section)

A delegation is encoded as follows:

| Name           | Size | Contents          |
|:---------------|:-----|:------------------|
| tag            | 1    | 0x6E              |
| source         | 21   | [Public Key Hash] |
| fee            |      | [Zarith]          |
| counter        |      | [Zarith]          |
| gas\_limit     |      | [Zarith]          |
| storage\_limit |      | [Zarith]          |
| delegate?      | 1    | [bool]            |
| delegate       |      | [Public Key Hash] |

## Primitive types

There are a couple of "primitive" types that make up the rest of the types

### Boolean

`tezos-codec describe ground.bool binary schema`

A boolean is encoded in a single byte with 0xFF if true or 0x00 if false

### Zarith

`tezos-codec describe ground.N binary schema`

Numbers are encoded usint the Zarith encoding method. This method encodes numbes as a variable
sequence of bytes, where the MSB of each byte determines whether the read byte is the last one (0)
or if there are more bytes to read (1).

After ignoring these MSBs, the data is then the binary representation of the absolute value
of the number in little endian order.

##### Signed

`tezos-codec describe ground.Z binary schema`

For signed numbers, the second MSB of the first byte is used to encode if the number is positive (0) or negative (1).

### Bytes

`tezos-codec describe ground.bytes binary schema`

A sequence of unknown lenght of bytes is prefixed with the sequence length in 4 bytes

### Float

`tezos-codec describe ground.float binary schema`

A float is just the IEEE 754 standard double-precision floating point representation and is 8 bytes long

### Public Key

`tezos-codec describe alpha.operation.contents` (search `public_key`)

There are 3 types of public keys, since Tezos support using ED25519, SECP256K1 and SECP256R1 (aka P256).

There are also 3 different type of hashes, one for each public key type.

When serialized, hashes and public keys are all prefixed with a tag.

| Tag  | Type      | Hash Len | Public Key Len |
| ---- | --------- | -------- | -------------- |
| 0x00 | Ed25519   | 20       | 32             |
| 0x01 | Secp256k1 | 20       | 33             |
| 0x02 | P256      | 20       | 33             |

### Contract ID

`tezos-codec describe alpha.operation.contents` (search `alpha.contract_id`)

Contract id is an encoding that represents either an implicit contract (wallet)
or an originated contract (smart contract).

These 2 are differentiated in encoding by a prefixed tag.

#### Implicit

| Name    | Size | Content           |
| :------ | :--- | :---------------- |
| Tag     | 1    | 0x00              |
| Address | 21   | [Public Key Hash] |

#### Originated

A transaction to an originated contract is a smart contract call,
see [Transaction parameters] for more info

Note: unspecified value for padding

| Name          | Size | Content |
| :------------ | :--- | :------ |
| Tag           | 1    | 0x01    |
| Contract Hash | 20   | [Bytes] |
| Padding       | 1    | 0x00    |

[endorsement]: (#endorsement)
[seed nonce revelation]: (#seed-nonce-revelation)
[double endorsement evidence]: (#double-endorsement-evidence)
[double baking evidence]: (#double-baking-evidence)
[activate account]: (#activate-account)
[proposals]: (#proposals)
[ballot]: (#ballot)
[endorsement with slot]: (#endorsement-with-slot)
[failing noop]: (#failing-noop)
[reveal]: (#reveal)
[transaction]: (#transaction)
[origination]: (#origination)
[delegation]: (#delegation)
[zarith]: (#zarith)
[transaction parameters]: (#parameters)
[public key hash]: (#public-key)
[public key]: (#public-key)
[bool]: (#boolean)
[bytes]: (#bytes)
[operations]: (#operation-types)
[entrypoint]: (#entrypoint)
[Vote]: (#vote)
[Script]: (#script)
[Inlined endorsement]: (#inlined-endorsement)
[Inlined block header]: (#inlined-block-header)
[fitness]: (#fitness)
