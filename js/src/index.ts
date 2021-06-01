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
import Transport from "@ledgerhq/hw-transport";
import { serializePath } from "./helper";
import { ResponseAddress, ResponseAppInfo, ResponseSign, ResponseVersion, ResponseGit, ResponseHWM, ResponseBase } from "./types";
import {
  CHUNK_SIZE,
  CLA,
  errorCodeToString,
  getVersion,
  INS,
  LedgerError,
  P1_VALUES,
  P2_CURVE,
  Curve,
  PAYLOAD_TYPE,
  PKLEN,
  processErrorResponse
} from "./common";

export { LedgerError, Curve };
export * from "./types";

function processGetAddrResponse(response: Buffer) {
  let partialResponse = response;

  const errorCodeData = partialResponse.slice(-2);
  const returnCode = (errorCodeData[0] * 256 + errorCodeData[1]);

  const publicKey = Buffer.from(partialResponse.slice(0, PKLEN));
  partialResponse = partialResponse.slice(PKLEN);

  const address = Buffer.from(partialResponse.slice(PKLEN, -2)).toString();

  return {
    publicKey,
    address,
    returnCode,
    errorMessage: errorCodeToString(returnCode)
  };
}

export default class TezosApp {
    transport;

  constructor(transport: Transport) {
    this.transport = transport;
    if (!transport) {
      throw new Error("Transport has not been defined");
    }
  }

  static prepareChunks(message: Buffer, serializedPathBuffer?: Buffer) {
    const chunks = [];

    // First chunk (only path)
    if (serializedPathBuffer !== undefined) {
      // First chunk (only path)
      chunks.push(serializedPathBuffer!);
    }

    const messageBuffer = Buffer.from(message);

    const buffer = Buffer.concat([messageBuffer]);
    for (let i = 0; i < buffer.length; i += CHUNK_SIZE) {
      let end = i + CHUNK_SIZE;
      if (i > buffer.length) {
        end = buffer.length;
      }
      chunks.push(buffer.slice(i, end));
    }

    return chunks;
  }

  async signGetChunks(path: string, message: Buffer) {
    return TezosApp.prepareChunks(message, serializePath(path));
  }

  async getVersion(): Promise<ResponseVersion> {
    return getVersion(this.transport).catch(err => processErrorResponse(err));
  }

  async getGit(): Promise<ResponseGit> {
    return this.transport.send(CLA, INS.GET_GIT, 0, 0).then(response => {
      const errorCodeData = response.slice(-2);
      const returnCode = (errorCodeData[0] * 256 + errorCodeData[1]) as LedgerError;

      return {
        returnCode,
        errorMessage: errorCodeToString(returnCode),
        commit_hash: response.slice(0, -2).toString('ascii'),
      }
    }, processErrorResponse)
  }

  async resetHighWatermark(level: number): Promise<ResponseBase> {
    let data = Buffer.allocUnsafe(4);
    data.writeInt32BE(level);

    return this.transport.send(CLA, INS.RESET_HWM, 0, 0, data).then(response => {
      const errorCodeData = response.slice(-2);
      const returnCode = (errorCodeData[0] * 256 + errorCodeData[1]) as LedgerError;

      return {
        returnCode,
        errorMessage: errorCodeToString(returnCode),
      }
    }, processErrorResponse)
  }

  async getHighWatermark(): Promise<ResponseHWM> {
    return this.transport.send(CLA, INS.GET_HWM, 0, 0).then(response => {
      const errorCodeData = response.slice(-2);
      const returnCode = (errorCodeData[0] * 256 + errorCodeData[1]) as LedgerError;

      const main = response.slice(0, -2).readInt32BE();

      return {
        returnCode,
        errorMessage: errorCodeToString(returnCode),
        main,
        test: null,
        chain_id: null
      }
    }, processErrorResponse)
  }

  async getAppInfo(): Promise<ResponseAppInfo> {
    return this.transport.send(0xb0, 0x01, 0, 0).then(response => {
      const errorCodeData = response.slice(-2);
      const returnCode = errorCodeData[0] * 256 + errorCodeData[1];

      const result: { errorMessage?: string; returnCode?: LedgerError } = {};

      let appName = "err";
      let appVersion = "err";
      let flagLen = 0;
      let flagsValue = 0;

      if (response[0] !== 1) {
        // Ledger responds with format ID 1. There is no spec for any format != 1
        result.errorMessage = "response format ID not recognized";
        result.returnCode = LedgerError.DeviceIsBusy;
      } else {
        const appNameLen = response[1];
        appName = response.slice(2, 2 + appNameLen).toString("ascii");
        let idx = 2 + appNameLen;
        const appVersionLen = response[idx];
        idx += 1;
        appVersion = response.slice(idx, idx + appVersionLen).toString("ascii");
        idx += appVersionLen;
        const appFlagsLen = response[idx];
        idx += 1;
        flagLen = appFlagsLen;
        flagsValue = response[idx];
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
        flagPINValidated: (flagsValue & 128) !== 0
      };
    }, processErrorResponse);
  }

  async getAddressAndPubKey(path: string, curve: Curve): Promise<ResponseAddress> {
    const serializedPath = serializePath(path);
    return this.transport
      .send(CLA, INS.GET_ADDR, P1_VALUES.ONLY_RETRIEVE, curve, serializedPath, [LedgerError.NoErrors])
      .then(processGetAddrResponse, processErrorResponse);
  }

  async showAddressAndPubKey(path: string, curve: Curve): Promise<ResponseAddress> {
    const serializedPath = serializePath(path);
    return this.transport
      .send(CLA, INS.GET_ADDR, P1_VALUES.SHOW_ADDRESS_IN_DEVICE, curve, serializedPath, [
        LedgerError.NoErrors
      ])
      .then(processGetAddrResponse, processErrorResponse);
  }

  async signSendChunk(chunkIdx: number, chunkNum: number, chunk: Buffer): Promise<ResponseSign> {
    let payloadType = PAYLOAD_TYPE.ADD;
    if (chunkIdx === 1) {
      payloadType = PAYLOAD_TYPE.INIT;
    }
    if (chunkIdx === chunkNum) {
      payloadType = PAYLOAD_TYPE.LAST;
    }

    return this.transport
      .send(CLA, INS.SIGN_SECP256K1, payloadType, 0, chunk, [
        LedgerError.NoErrors,
        LedgerError.DataIsInvalid,
        LedgerError.BadKeyHandle,
        LedgerError.SignVerifyError
      ])
      .then((response: Buffer) => {
        const errorCodeData = response.slice(-2);
        const returnCode = errorCodeData[0] * 256 + errorCodeData[1];
        let errorMessage = errorCodeToString(returnCode);

        let signature = Buffer.alloc(0);

        if (returnCode === LedgerError.BadKeyHandle ||
          returnCode === LedgerError.DataIsInvalid ||
          returnCode === LedgerError.SignVerifyError) {
          errorMessage = `${errorMessage} : ${response
            .slice(0, response.length - 2)
            .toString("ascii")}`;
        }

        if (returnCode === LedgerError.NoErrors && response.length > 2) {
          signature = response.slice(32, 97);
          return {
            signature,
            returnCode: returnCode,
            errorMessage: errorMessage
          };
        }

        return {
          returnCode: returnCode,
          errorMessage: errorMessage
        };

      }, processErrorResponse);
  }


  async sign(path: string, message: Buffer) {
    return this.signGetChunks(path, message).then(chunks => {
      return this.signSendChunk(1, chunks.length, chunks[0]).then(async response => {
        let result = {
          returnCode: response.returnCode,
          errorMessage: response.errorMessage,
          signature: null as null | Buffer,
        };
        for (let i = 1; i < chunks.length; i += 1) {
          // eslint-disable-next-line no-await-in-loop
          result = await this.signSendChunk(1 + i, chunks.length, chunks[i]);
          if (result.returnCode !== LedgerError.NoErrors) {
            break;
          }
        }
        return result;
      }, processErrorResponse);
    }, processErrorResponse);
  }
}
