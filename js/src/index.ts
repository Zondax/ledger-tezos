/** ******************************************************************************
 *  (c) 2019-2020 Zondax GmbH
 *  (c) 2016-2017 Ledger
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 ******************************************************************************* */
import Transport from '@ledgerhq/hw-transport'
import { serializePath, sha256x2 } from './helper'
import {
  ResponseAddress,
  ResponseAppInfo,
  ResponseBase,
  ResponseHMAC,
  ResponseLegacyGit,
  ResponseLegacyHWM,
  ResponseLegacyVersion,
  ResponseQueryAuthKey,
  ResponseSign,
  ResponseVersion,
} from './types'
import {
  CHUNK_SIZE,
  CLA,
  Curve,
  errorCodeToString,
  getVersion,
  INS,
  LedgerError,
  LEGACY_INS,
  P1_VALUES,
  PAYLOAD_TYPE,
  processErrorResponse,
} from './common'

import { blake2b } from 'hash-wasm'

export { LedgerError, Curve }
export * from './types'

function processGetAddrResponse(response: Buffer) {
  let partialResponse = response

  const errorCodeData = partialResponse.slice(-2)
  const returnCode = errorCodeData[0] * 256 + errorCodeData[1]

  //get public key len (variable)
  const PKLEN = partialResponse[0]
  const publicKey = Buffer.from(partialResponse.slice(1, 1 + PKLEN))

  //"advance" buffer
  partialResponse = partialResponse.slice(1 + PKLEN)

  const address = Buffer.from(partialResponse.slice(0, -2)).toString()

  return {
    publicKey,
    address,
    returnCode,
    errorMessage: errorCodeToString(returnCode),
  }
}

function processAuthorizeBakingResponse(response: Buffer) {
  let partialResponse = response

  const errorCodeData = partialResponse.slice(-2)
  const returnCode = errorCodeData[0] * 256 + errorCodeData[1]

  //get public key len (variable)
  const PKLEN = partialResponse[0]
  const publicKey = Buffer.from(partialResponse.slice(1, 1 + PKLEN))

  return {
    publicKey,
    returnCode,
    errorMessage: errorCodeToString(returnCode),
  }
}

function processDeAuthorizeBakingResponse(response: Buffer) {
  let partialResponse = response

  const errorCodeData = partialResponse.slice(-2)
  const returnCode = errorCodeData[0] * 256 + errorCodeData[1]

  return {
    returnCode,
    errorMessage: errorCodeToString(returnCode),
  }
}

function processQueryAuthKeyWithCurve(response: Buffer) {
  let partialResponse = response

  const errorCodeData = partialResponse.slice(-2)
  const returnCode = errorCodeData[0] * 256 + errorCodeData[1]

  //get public key len (variable)
  const curve = partialResponse[0]
  const len_path = partialResponse[1]
  const bip32 = partialResponse.slice(2, 2 + 4 * len_path)

  return {
    curve,
    bip32,
    returnCode,
    errorMessage: errorCodeToString(returnCode),
  }
}

export default class TezosApp {
  transport

  constructor(transport: Transport) {
    this.transport = transport
    if (!transport) {
      throw new Error('Transport has not been defined')
    }
  }

  static prepareChunks(message: Buffer, serializedPathBuffer?: Buffer) {
    const chunks = []

    // First chunk (only path)
    if (serializedPathBuffer !== undefined) {
      // First chunk (only path)
      chunks.push(serializedPathBuffer!)
    }

    const messageBuffer = Buffer.from(message)

    const buffer = Buffer.concat([messageBuffer])
    for (let i = 0; i < buffer.length; i += CHUNK_SIZE) {
      let end = i + CHUNK_SIZE
      if (i > buffer.length) {
        end = buffer.length
      }
      chunks.push(buffer.slice(i, end))
    }

    return chunks
  }

  async signGetChunks(path: string, message: Buffer) {
    return TezosApp.prepareChunks(message, serializePath(path))
  }

  async getVersion(): Promise<ResponseVersion> {
    return getVersion(this.transport).catch(err => processErrorResponse(err))
  }

  async getAppInfo(): Promise<ResponseAppInfo> {
    return this.transport.send(0xb0, 0x01, 0, 0).then(response => {
      const errorCodeData = response.slice(-2)
      const returnCode = errorCodeData[0] * 256 + errorCodeData[1]

      const result: { errorMessage?: string; returnCode?: LedgerError } = {}

      let appName = 'err'
      let appVersion = 'err'
      let flagLen = 0
      let flagsValue = 0

      if (response[0] !== 1) {
        // Ledger responds with format ID 1. There is no spec for any format != 1
        result.errorMessage = 'response format ID not recognized'
        result.returnCode = LedgerError.DeviceIsBusy
      } else {
        const appNameLen = response[1]
        appName = response.slice(2, 2 + appNameLen).toString('ascii')
        let idx = 2 + appNameLen
        const appVersionLen = response[idx]
        idx += 1
        appVersion = response.slice(idx, idx + appVersionLen).toString('ascii')
        idx += appVersionLen
        const appFlagsLen = response[idx]
        idx += 1
        flagLen = appFlagsLen
        flagsValue = response[idx]
      }

      return {
        returnCode,
        errorMessage: errorCodeToString(returnCode),
        //
        appName,
        appVersion,
        flagLen,
        flagsValue,
        flagRecovery: (flagsValue & 1) !== 0,
        // eslint-disable-next-line no-bitwise
        flagSignedMcuCode: (flagsValue & 2) !== 0,
        // eslint-disable-next-line no-bitwise
        flagOnboarded: (flagsValue & 4) !== 0,
        // eslint-disable-next-line no-bitwise
        flagPINValidated: (flagsValue & 128) !== 0,
      }
    }, processErrorResponse)
  }

  async getAddressAndPubKey(path: string, curve: Curve): Promise<ResponseAddress> {
    const serializedPath = serializePath(path)
    return this.transport
      .send(CLA, INS.GET_ADDR, P1_VALUES.ONLY_RETRIEVE, curve, serializedPath, [LedgerError.NoErrors])
      .then(processGetAddrResponse, processErrorResponse)
  }

  async showAddressAndPubKey(path: string, curve: Curve): Promise<ResponseAddress> {
    const serializedPath = serializePath(path)
    return this.transport
      .send(CLA, INS.GET_ADDR, P1_VALUES.SHOW_ADDRESS_IN_DEVICE, curve, serializedPath, [LedgerError.NoErrors])
      .then(processGetAddrResponse, processErrorResponse)
  }

  async authorizeBaking(path: string, curve: Curve): Promise<ResponseAddress> {
    const serializedPath = serializePath(path)
    return this.transport
      .send(CLA, INS.AUTHORIZE_BAKING, 0x01, curve, serializedPath, [LedgerError.NoErrors])
      .then(processAuthorizeBakingResponse, processErrorResponse)
  }

  async deauthorizeBaking(): Promise<ResponseBase> {
    return this.transport
      .send(CLA, INS.DEAUTHORIZE_BAKING, 0x01, 0x00, Buffer.alloc(0), [LedgerError.NoErrors])
      .then(processDeAuthorizeBakingResponse, processErrorResponse)
  }

  async queryAuthKeyWithCurve(confirm = false): Promise<ResponseQueryAuthKey> {
    const confirmation = confirm ? 1 : 0

    return this.transport
      .send(CLA, INS.QUERY_AUTH_KEY_WITH_CURVE, confirmation, 0x00, Buffer.alloc(0), [LedgerError.NoErrors])
      .then(processQueryAuthKeyWithCurve, processErrorResponse)
  }

  async signSendChunk(
    chunkIdx: number,
    chunkNum: number,
    chunk: Buffer,
    legacy = false,
    curve?: Curve,
    ins: number = INS.SIGN,
    with_hash = true,
  ): Promise<ResponseSign> {
    let payloadType = PAYLOAD_TYPE.ADD
    let p2 = 0
    if (chunkIdx === 1) {
      payloadType = PAYLOAD_TYPE.INIT
      if (curve === undefined) {
        throw Error('curve type not given')
      }
      p2 = curve
    }
    if (chunkIdx === chunkNum) {
      if (!legacy) {
        payloadType = PAYLOAD_TYPE.LAST
      } else {
        //when legacy, mark as last instead of setting last
        payloadType |= 0x80
      }
    }

    return this.transport
      .send(CLA, ins, payloadType, p2, chunk, [
        LedgerError.NoErrors,
        LedgerError.DataIsInvalid,
        LedgerError.BadKeyHandle,
        LedgerError.SignVerifyError,
      ])
      .then((response: Buffer) => {
        const errorCodeData = response.slice(-2)
        const returnCode = errorCodeData[0] * 256 + errorCodeData[1]
        let errorMessage = errorCodeToString(returnCode)

        if (
          returnCode === LedgerError.BadKeyHandle ||
          returnCode === LedgerError.DataIsInvalid ||
          returnCode === LedgerError.SignVerifyError
        ) {
          errorMessage = `${errorMessage} : ${response.slice(0, response.length - 2).toString('ascii')}`
        }

        if (returnCode === LedgerError.NoErrors && response.length > 2) {
          if (with_hash) {
            return {
              hash: response.slice(0, 32),
              signature: response.slice(32, -2),
              returnCode: returnCode,
              errorMessage: errorMessage,
            }
          } else {
            return {
              signature: response.slice(0, 32),
              hash: Buffer.alloc(32),
              returnCode: returnCode,
              errorMessage: errorMessage,
            }
          }
        }

        return {
          returnCode: returnCode,
          errorMessage: errorMessage,
        }
      }, processErrorResponse)
  }

  async signBaker(path: string, curve: Curve, message: Buffer, message_type: 'endorsement' | 'blocklevel' | 'delegation') {
    //we prepend the appropriate "magic byte"
    //based on the type of the content that we want to sign
    let magic_byte
    switch (message_type) {
      case 'blocklevel':
        const block_protocol_version_offset = 4 + 4 + 1 + 32 + 8 + 1 + 32 + 4;
        switch (message[block_protocol_version_offset]) {
          case 0:
          case 1:
            //emmy block
            magic_byte = 0x01;
            break;
          case 2:
            //tenderbake block
            magic_byte = 0x11
            break;
          default:
            throw "unknown block protocol version"
        }
        break
      case 'endorsement':
        const endorsement_tag = message[4 + 32];
        switch (endorsement_tag) {
          case 20:
            magic_byte = 0x12 //pre endorsement
            break;
          case 21:
            magic_byte = 0x13 //endorsement
            break;
          default:
            magic_byte = 0x02 //emmy endorsement
            break;
        }
        break;
      case 'delegation':
        magic_byte = 3
        break;
      default:
        throw 'Invalid message type'
    }
    message = Buffer.concat([Buffer.from([magic_byte]), message])

    return this.signGetChunks(path, message).then(chunks => {
      return this.signSendChunk(1, chunks.length, chunks[0], false, curve, INS.BAKER_SIGN).then(async response => {
        let result = {
          returnCode: response.returnCode,
          errorMessage: response.errorMessage,
          signature: null as null | Buffer,
        }
        for (let i = 1; i < chunks.length; i += 1) {
          // eslint-disable-next-line no-await-in-loop
          result = await this.signSendChunk(1 + i, chunks.length, chunks[i], false, curve, INS.BAKER_SIGN)
          if (result.returnCode !== LedgerError.NoErrors) {
            break
          }
        }
        return result
      }, processErrorResponse)
    }, processErrorResponse)
  }

  async signOperation(path: string, curve: Curve, message: Buffer) {
    //prepend 0x03 to signal an operation as the message
    message = Buffer.concat([Buffer.from([3]), message])

    return this.signGetChunks(path, message).then(chunks => {
      return this.signSendChunk(1, chunks.length, chunks[0], false, curve, INS.SIGN).then(async response => {
        let result = {
          returnCode: response.returnCode,
          errorMessage: response.errorMessage,
          signature: null as null | Buffer,
        }
        for (let i = 1; i < chunks.length; i += 1) {
          // eslint-disable-next-line no-await-in-loop
          result = await this.signSendChunk(1 + i, chunks.length, chunks[i], false, curve, INS.SIGN)
          if (result.returnCode !== LedgerError.NoErrors) {
            break
          }
        }
        return result
      }, processErrorResponse)
    }, processErrorResponse)
  }

  async signMichelson(path: string, curve: Curve, message: Buffer) {
    //prepend 0x05 to signal some packed michelson data
    message = Buffer.concat([Buffer.from([5]), message])

    return this.signGetChunks(path, message).then(chunks => {
      return this.signSendChunk(1, chunks.length, chunks[0], false, curve, INS.SIGN).then(async response => {
        let result = {
          returnCode: response.returnCode,
          errorMessage: response.errorMessage,
          signature: null as null | Buffer,
        }
        for (let i = 1; i < chunks.length; i += 1) {
          // eslint-disable-next-line no-await-in-loop
          result = await this.signSendChunk(1 + i, chunks.length, chunks[i], false, curve, INS.SIGN)
          if (result.returnCode !== LedgerError.NoErrors) {
            break
          }
        }
        return result
      }, processErrorResponse)
    }, processErrorResponse)
  }

  async sig_hash(msg: Buffer, msg_type: 'blocklevel' | 'endorsement' | 'operation' | 'michelson'): Promise<Buffer> {
    let magic_byte
    switch (msg_type) {
      case 'blocklevel':
        magic_byte = 1
        break
      case 'endorsement':
        magic_byte = 2
        break
      case 'operation':
        magic_byte = 3
        break
      case 'michelson':
        magic_byte = 5
        break
      default:
        throw 'Invalid message type'
    }
    msg = Buffer.concat([Buffer.from([magic_byte]), msg])

    const hexHash = await blake2b(msg, 32 * 8);

    return Buffer.from(hexHash, 'hex');
  }

  //--------------------- LEGACY INSTRUCTIONS
  async legacyGetVersion(): Promise<ResponseLegacyVersion> {
    return this.transport.send(CLA, LEGACY_INS.VERSION, 0, 0).then(response => {
      const errorCodeData = response.slice(-2)
      const returnCode = (errorCodeData[0] * 256 + errorCodeData[1]) as LedgerError

      return {
        returnCode,
        errorMessage: errorCodeToString(returnCode),
        baking: response[0] == 1,
        major: response[1],
        minor: response[2],
        patch: response[3],
      }
    }, processErrorResponse)
  }

  async legacyGetGit(): Promise<ResponseLegacyGit> {
    return this.transport.send(CLA, LEGACY_INS.GIT, 0, 0).then(response => {
      const errorCodeData = response.slice(-2)
      const returnCode = (errorCodeData[0] * 256 + errorCodeData[1]) as LedgerError

      return {
        returnCode,
        errorMessage: errorCodeToString(returnCode),
        commit_hash: response.slice(0, -2).toString('ascii'),
      }
    }, processErrorResponse)
  }

  async legacyResetHighWatermark(level: number): Promise<ResponseBase> {
    const data = Buffer.allocUnsafe(4)
    data.writeInt32BE(level)

    return this.transport.send(CLA, LEGACY_INS.RESET, 0, 0, data).then(response => {
      const errorCodeData = response.slice(-2)
      const returnCode = (errorCodeData[0] * 256 + errorCodeData[1]) as LedgerError

      return {
        returnCode,
        errorMessage: errorCodeToString(returnCode),
      }
    }, processErrorResponse)
  }

  async legacyGetHighWatermark(): Promise<ResponseLegacyHWM> {
    return this.transport.send(CLA, LEGACY_INS.QUERY_MAIN_HWM, 0, 0).then(response => {
      const errorCodeData = response.slice(-2)
      const returnCode = (errorCodeData[0] * 256 + errorCodeData[1]) as LedgerError

      const main = response.slice(0, -2).readUInt32BE()

      return {
        returnCode,
        errorMessage: errorCodeToString(returnCode),
        main,
        test: null,
        chain_id: null,
      }
    }, processErrorResponse)
  }

  async legacyGetAllWatermark(): Promise<ResponseLegacyHWM> {
    return this.transport.send(CLA, LEGACY_INS.QUERY_ALL_HWM, 0, 0).then(response => {
      const errorCodeData = response.slice(-2)
      const returnCode = (errorCodeData[0] * 256 + errorCodeData[1]) as LedgerError

      const main = response.slice(0, 4).readUInt32BE()
      const test = response.slice(4, 8).readUInt32BE()
      const chain_id = response.slice(8, -2).readUInt32BE()

      return {
        returnCode,
        errorMessage: errorCodeToString(returnCode),
        main,
        test,
        chain_id,
      }
    }, processErrorResponse)
  }

  async legacySetup(path: string, curve: Curve, main_level: number, test_level: number, chain_id = 0x7a06a770): Promise<ResponseAddress> {
    const serializedPath = serializePath(path)

    let data = Buffer.allocUnsafe(4 * 3)
    data.writeUInt32BE(chain_id)
    data.writeUInt32BE(main_level, 4)
    data.writeUInt32BE(test_level, 8)
    data = Buffer.concat([data, serializedPath])

    return this.transport.send(CLA, LEGACY_INS.SETUP, 0, curve, data).then(async (response) => {
      const errorCodeData = response.slice(-2)
      const returnCode = (errorCodeData[0] * 256 + errorCodeData[1]) as LedgerError

      const publicKey = response.slice(0, -2)
      const address = await this.publicKeyToAddress(publicKey, curve)

      return {
        returnCode,
        errorMessage: errorCodeToString(returnCode),
        publicKey,
        address,
      }
    }, processErrorResponse)
  }

  async legacyHMAC(path: string, curve: Curve, message: Buffer): Promise<ResponseHMAC> {
    const serializedPath = serializePath(path)
    return this.transport.send(CLA, LEGACY_INS.HMAC, 0, curve, Buffer.concat([serializedPath, message])).then(async (response) => {
      const errorCodeData = response.slice(-2)
      const returnCode = (errorCodeData[0] * 256 + errorCodeData[1]) as LedgerError

      const hmac = response.slice(0, -2)

      return {
        returnCode,
        errorMessage: errorCodeToString(returnCode),
        hmac,
      }
    }, processErrorResponse)
  }

  async legacyGetPubKey(path: string, curve: Curve): Promise<ResponseAddress> {
    const serializedPath = serializePath(path)
    return this.transport.send(CLA, LEGACY_INS.PUBLIC_KEY, P1_VALUES.ONLY_RETRIEVE, curve, serializedPath).then(async (response) => {
      const errorCodeData = response.slice(-2)
      const returnCode = (errorCodeData[0] * 256 + errorCodeData[1]) as LedgerError

      const publicKey = response.slice(0, -2)
      const address = await this.publicKeyToAddress(publicKey, curve)

      return {
        returnCode,
        errorMessage: errorCodeToString(returnCode),
        publicKey,
        address,
      }
    }, processErrorResponse)
  }

  async legacyPromptPubKey(path: string, curve: Curve): Promise<ResponseAddress> {
    const serializedPath = serializePath(path)
    return this.transport.send(CLA, LEGACY_INS.PROMPT_PUBLIC_KEY, 0, curve, serializedPath).then(async (response) => {
      const errorCodeData = response.slice(-2)
      const returnCode = (errorCodeData[0] * 256 + errorCodeData[1]) as LedgerError

      const publicKey = response.slice(0, -2)
      const address = await this.publicKeyToAddress(publicKey, curve)

      return {
        returnCode,
        errorMessage: errorCodeToString(returnCode),
        publicKey,
        address,
      }
    }, processErrorResponse)
  }

  async publicKeyToAddress(key: Buffer, curve: Curve): Promise<string> {
    let prefix

    switch (curve) {
      case Curve.Ed25519:
      case Curve.Ed25519_Slip10:
        prefix = [6, 161, 159]
        break

      case Curve.Secp256K1:
        prefix = [6, 161, 161]
        break

      case Curve.Secp256R1:
        prefix = [6, 161, 164]
        break

      default:
        throw Error('not a valid curve type')
    }

    prefix = Buffer.from(prefix)

    switch (curve) {
      case Curve.Ed25519:
      case Curve.Ed25519_Slip10:
        key = key.slice(1) //this skips the len byte
        break

      case Curve.Secp256K1:
      case Curve.Secp256R1:
        const last = key.readUInt8(64)
        key = key.slice(0, 33) //we actually override [0] down below
        key.writeUInt8(0x02 + (last & 0x01))
        break
    }

    const hexHash = await blake2b(key, 20 * 8)
    const hash = Buffer.from(hexHash, 'hex');

    const checksum = sha256x2(Buffer.concat([prefix, hash])).slice(0, 4)

    const bs58 = require('bs58')

    return bs58.encode(Buffer.concat([prefix, hash, checksum]))
  }

  async legacySignWithHash(
    path: string,
    curve: Curve,
    message: Buffer,
    message_type: 'endorsement' | 'blocklevel' | 'operation' | 'michelson' = 'operation',
  ) {
    //we prepend the appropriate "magic byte"
    //based on the type of the content that we want to sign
    let magic_byte
    switch (message_type) {
      case 'blocklevel':
        magic_byte = 1
        break
      case 'endorsement':
        magic_byte = 2
        break
      case 'operation':
        magic_byte = 3
        break
      case 'michelson':
        magic_byte = 5
        break
      default:
        throw 'Invalid message type'
    }
    message = Buffer.concat([Buffer.from([magic_byte]), message])

    return this.signGetChunks(path, message).then(chunks => {
      return this.signSendChunk(1, chunks.length, chunks[0], true, curve, LEGACY_INS.SIGN_WITH_HASH).then(async response => {
        let result = {
          returnCode: response.returnCode,
          errorMessage: response.errorMessage,
          signature: null as null | Buffer,
        }
        for (let i = 1; i < chunks.length; i += 1) {
          // eslint-disable-next-line no-await-in-loop
          result = await this.signSendChunk(1 + i, chunks.length, chunks[i], true, curve, LEGACY_INS.SIGN_WITH_HASH)
          if (result.returnCode !== LedgerError.NoErrors) {
            break
          }
        }
        return result
      }, processErrorResponse)
    }, processErrorResponse)
  }

  async legacySign(
    path: string,
    curve: Curve,
    message: Buffer,
    message_type: 'endorsement' | 'blocklevel' | 'operation' | 'michelson' = 'operation',
  ) {
    //we prepend the appropriate "magic byte"
    //based on the type of the content that we want to sign
    let magic_byte
    switch (message_type) {
      case 'blocklevel':
        magic_byte = 1
        break
      case 'endorsement':
        magic_byte = 2
        break
      case 'operation':
        magic_byte = 3
        break
      case 'michelson':
        magic_byte = 5
        break
      default:
        throw 'Invalid message type'
    }
    message = Buffer.concat([Buffer.from([magic_byte]), message])

    return this.signGetChunks(path, message).then(chunks => {
      return this.signSendChunk(1, chunks.length, chunks[0], true, curve, LEGACY_INS.SIGN, false).then(async response => {
        let result = {
          returnCode: response.returnCode,
          errorMessage: response.errorMessage,
          signature: null as null | Buffer,
        }
        for (let i = 1; i < chunks.length; i += 1) {
          // eslint-disable-next-line no-await-in-loop
          result = await this.signSendChunk(1 + i, chunks.length, chunks[i], true, curve, LEGACY_INS.SIGN, false)
          if (result.returnCode !== LedgerError.NoErrors) {
            break
          }
        }
        return result
      }, processErrorResponse)
    }, processErrorResponse)
  }
}
