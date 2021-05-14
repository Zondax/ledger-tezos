import TezosApp, { LedgerError, ResponseBase } from '@zondax/ledger-tezos'
import { errorCodeToString, processErrorResponse, CLA } from '@zondax/ledger-tezos/dist/common'

const INS = {
  EXCEPT: 0xf1,
}

interface ResponseException extends ResponseBase {
  ex: number
}

export default class TezosAppDev extends TezosApp {
  async except(should_catch: boolean, ex: number): Promise<ResponseException> {
    return this.transport.send(CLA, INS.EXCEPT, Number(should_catch), ex).then(response => {
      const errorCodeData = response.slice(-2)
      const returnCode = (errorCodeData[0] * 256 + errorCodeData[1]) as LedgerError

      return {
        returnCode,
        errorMessage: errorCodeToString(returnCode),
        ex: response.slice(0, -2).readInt16BE(),
      }
    }, processErrorResponse)
  }
}
