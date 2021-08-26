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
import Zemu from '@zondax/zemu'

import { TestVector } from '../test-vectors-gen/legacy'

import { readdir, readFile, writeFile } from 'fs'
import { promisify } from 'util'

async function readTestVectors(): Promise<TestVector[]> {
  const vectors: TestVector[] = []

  await promisify(readdir)('test-vectors/')
    .then(filenames => {
      return Promise.all(
        filenames.map(filename => {
          return promisify(readFile)('test-vectors/' + filename, { encoding: 'utf8' })
        }),
      )
    })
    .then(allContents => {
      allContents.forEach(contents => {
        vectors.push(...JSON.parse(contents))
      })
    })
    .catch(_ => {
      return;
    })

  return vectors
}

module.exports = async () => {
  await Zemu.checkAndPullImage()

  const writeFilePr = promisify(writeFile)

  const vectors = await readTestVectors()
  await writeFilePr('/tmp/jest.collected_test_vectors.json', JSON.stringify(vectors)).catch(err => {
    throw err
  })
}
