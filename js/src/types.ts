export interface ResponseBase {
  errorMessage: string
  returnCode: number
}

export interface ResponseAddress extends ResponseBase {
  publicKey: Buffer
  address: string
}

export interface ResponseQueryAuthKey extends ResponseBase {
  publicKey: Buffer
  curve: number
}

export interface ResponseVersion extends ResponseBase {
  testMode: boolean
  major: number
  minor: number
  patch: number
  deviceLocked: boolean
  targetId: string
}

export interface ResponseAppInfo extends ResponseBase {
  appName: string
  appVersion: string
  flagLen: number
  flagsValue: number
  flagRecovery: boolean
  flagSignedMcuCode: boolean
  flagOnboarded: boolean
  flagPINValidated: boolean
}

export interface ResponseSign extends ResponseBase {
  hash: Buffer
  signature: Buffer
}

//------------------------ LEGACY RESPONSES

export interface ResponseLegacyGit extends ResponseBase {
  commit_hash: string
}

export interface ResponseLegacyVersion extends ResponseBase {
  baking: boolean
  major: number
  minor: number
  patch: number
}

export interface ResponseLegacyHWM extends ResponseBase {
  main: number
  test?: number
  chain_id?: number
}

export interface ResponseHMAC extends ResponseBase {
  hmac: Buffer
}
