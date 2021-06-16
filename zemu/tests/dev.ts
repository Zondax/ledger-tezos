import TezosApp, { LedgerError, ResponseBase } from '@zondax/ledger-tezos'
import { errorCodeToString, processErrorResponse, CLA } from '@zondax/ledger-tezos/dist/common'

const INS = {
  HASH: 0xf0,
  EXCEPT: 0xf1,
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
}
