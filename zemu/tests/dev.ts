import TezosApp, { Curve, LedgerError, ResponseBase } from '@zondax/ledger-tezos'
import { CLA, errorCodeToString, processErrorResponse } from '@zondax/ledger-tezos/dist/common'

const INS = {
  HASH: 0xf0,
  EXCEPT: 0xf1,
  ECHO: 0xf2,
  SIGN: 0xf3
}

interface ResponseHash extends ResponseBase {
  hash: null | Buffer
}

interface ResponseException extends ResponseBase {
  ex: BigInt
}

export default class TezosAppDev extends TezosApp {
  async except(should_catch: boolean, ex: number): Promise<ResponseException> {
    return this.transport.send(CLA, INS.EXCEPT, Number(should_catch), ex).then(response => {
      const errorCodeData = response.slice(-2)
      const returnCode = (errorCodeData[0] * 256 + errorCodeData[1]) as LedgerError

      return {
        returnCode,
        errorMessage: errorCodeToString(returnCode),
        ex: response.slice(0, -2).readBigInt64BE(),
      }
    }, processErrorResponse)
  }

  async sendHashChunk(idx: number, max: number, chunk: Buffer): Promise<ResponseHash> {
    let payloadType = 0x01
    if (idx === 1) {
      payloadType = 0x00
    }
    if (idx === max) {
      payloadType = 0x02
    }

    return this.transport.send(0x80, INS.HASH, payloadType, 0, chunk).then(response => {
      const errorCodeData = response.slice(-2)
      const returnCode = errorCodeData[0] * 256 + errorCodeData[1]

      let hash: null | Buffer = null
      if (returnCode === LedgerError.NoErrors && response.length > 2) {
        hash = response.slice(0, 32)
      }

      return {
        hash,
        returnCode,
        errorMessage: errorCodeToString(returnCode),
      }
    })
  }

  async getHash(message: Buffer): Promise<ResponseHash> {
    const chunks = TezosApp.prepareChunks(message)

    let result = {
      hash: null as null | Buffer,
      returnCode: LedgerError.UnknownError,
      errorMessage: errorCodeToString(LedgerError.UnknownError),
    }

    for (let i = 0; i < chunks.length; i += 1) {
      result = await this.sendHashChunk(i, chunks.length - 1, chunks[i])
      if (result.hash !== null) {
        break
      }
    }

    return result
  }

  async display(message: Buffer): Promise<ResponseBase> {
    if (message.length > 35) {
      throw Error('message too long')
    }

    return this.transport.send(CLA, INS.ECHO, 0, 0, message).then(response => {
      const errorCodeData = response.slice(-2)
      const returnCode = (errorCodeData[0] * 256 + errorCodeData[1]) as LedgerError

      return {
        returnCode,
        errorMessage: errorCodeToString(returnCode),
      }
    }, processErrorResponse)
  }

  async blind_sign(path: string, curve: Curve, message: Buffer) {
    //prepend 0x05 to signal a michelson packed struct
    // which is signed blindly based on the hash only
    message = Buffer.concat([Buffer.from([5]), message])

    return this.signGetChunks(path, message).then(chunks => {
      return this.signSendChunk(1, chunks.length, chunks[0], false, curve, INS.SIGN).then(async response => {
        let result = {
          returnCode: response.returnCode,
          errorMessage: response.errorMessage,
          signature: null as null | Buffer,
        };
        for (let i = 1; i < chunks.length; i += 1) {
          // eslint-disable-next-line no-await-in-loop
          result = await this.signSendChunk(1 + i, chunks.length, chunks[i], false, curve, INS.SIGN);
          if (result.returnCode !== LedgerError.NoErrors) {
            break;
          }
        }
        return result;
      }, processErrorResponse);
    }, processErrorResponse);
  }
}
