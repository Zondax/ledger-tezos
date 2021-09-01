/** ******************************************************************************
 *  (c) 2020 Zondax GmbH
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
import { readFileSync } from 'fs'
import { resolve } from 'path'

const dataPath = resolve(__dirname, 'data')

export const SAMPLE_OPERATIONS: { op: any; blob: string }[] = JSON.parse(readFileSync(dataPath + '/samples.json', 'utf-8'))
export const SAMPLE_CONTRACTS: { op: any; blob: string }[] = JSON.parse(readFileSync(dataPath + '/michelson.json', 'utf-8'))

export const SAMPLE_TRANSACTION = SAMPLE_OPERATIONS[6]
export const SAMPLE_DELEGATION = SAMPLE_OPERATIONS[0]
export const SAMPLE_ENDORSEMENT = SAMPLE_OPERATIONS[3]
export const SAMPLE_SEED_NONCE_REVELATION = SAMPLE_OPERATIONS[4]
export const SAMPLE_BALLOT = SAMPLE_OPERATIONS[2]
export const SAMPLE_REVEAL = SAMPLE_OPERATIONS[1]
export const SAMPLE_PROPOSALS = SAMPLE_OPERATIONS[5]
export const SAMPLE_ORIGINATION = SAMPLE_OPERATIONS[20];
