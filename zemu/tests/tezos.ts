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
import { localForger } from '@taquito/local-forging'

//source: https://github.com/ecadlabs/taquito/blob/master/packages/taquito-local-forging/test/allTestsCases.ts
export const SAMPLE_OPERATIONS_NO_BLOB = [
  {
    name: 'Delegation',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'delegation',
          delegate: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
        },
      ],
    },
  },
  {
    name: 'Reveal',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'reveal',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          public_key: 'edpkvS5QFv7KRGfa3b87gg9DBpxSm3NpSwnjhUjNBQrRUUR66F7C9g',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
        },
      ],
    },
  },
  {
    name: 'Ballot',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'ballot',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          period: -300,
          ballot: 'yay',
          proposal: 'PsCARTHAGazKbHtnKfLzQg3kms52kSRpgnDY982a9oYsSXRLQEb',
        },
      ],
    },
  },
  {
    name: 'Endorsement',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'endorsement',
          level: -300,
        },
      ],
    },
  },
  {
    name: 'Seed nonce revelation',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'seed_nonce_revelation',
          level: 25550,
          nonce: new Array(32).fill('ff').join(''),
        },
      ],
    },
  },
  {
    name: 'Proposals',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'proposals',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          period: 25550,
          proposals: ['PsCARTHAGazKbHtnKfLzQg3kms52kSRpgnDY982a9oYsSXRLQEb', 'PsCARTHAGazKbHtnKfLzQg3kms52kSRpgnDY982a9oYsSXRLQEb'],
        },
      ],
    },
  },
  {
    name: 'Transaction',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'transaction',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
          destination: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          amount: '1000',
        },
      ],
    },
  },
  {
    name: 'Transaction with parameter',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'transaction',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
          parameters: {
            entrypoint: 'do',
            value: { bytes: '0202' },
          },
          destination: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          amount: '1000',
        },
      ],
    },
  },
  {
    name: 'Transaction with default entrypoint',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'transaction',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
          parameters: {
            entrypoint: 'default',
            value: { prim: 'Pair', args: [{ int: '2' }, { string: 'hello' }] },
          },
          destination: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          amount: '1000',
        },
      ],
    },
  },
  {
    name: 'Transaction with maximum length entrypoint',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'transaction',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
          parameters: {
            entrypoint: 'Tps0RV2UISBvTV6m2z16VnfCVnN5dzX',
            value: { prim: 'Pair', args: [{ int: '2' }, { string: 'hello' }] },
          },
          destination: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          amount: '1000',
        },
      ],
    },
  },
  {
    name: 'Transaction with non ascii entrypoint and string',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'transaction',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
          parameters: {
            entrypoint: 'entrypoint©Ͻ',
            value: { string: 'Copyright ©Ͻ' },
          },
          destination: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          amount: '1000',
        },
      ],
    },
  },
  {
    name: 'Transaction with default entrypoint and unit parameter',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'transaction',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
          parameters: {
            entrypoint: 'default',
            value: { prim: 'Unit' },
          },
          destination: 'KT1JHqHQdHSgWBKo6H4UfG8dw3JnZSyjGkHA',
          amount: '1000',
        },
      ],
    },
    expected: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'transaction',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
          destination: 'KT1JHqHQdHSgWBKo6H4UfG8dw3JnZSyjGkHA',
          amount: '1000',
        },
      ],
    },
  },
  {
    name: 'Transaction with set_delegate entrypoint',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'transaction',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
          parameters: {
            entrypoint: 'set_delegate',
            value: { string: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn' },
          },
          destination: 'KT1JHqHQdHSgWBKo6H4UfG8dw3JnZSyjGkHA',
          amount: '1000',
        },
      ],
    },
  },
  {
    name: 'Transaction with remove_delegate entrypoint',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'transaction',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
          parameters: {
            entrypoint: 'remove_delegate',
            value: { string: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn' },
          },
          destination: 'KT1JHqHQdHSgWBKo6H4UfG8dw3JnZSyjGkHA',
          amount: '1000',
        },
      ],
    },
  },
  {
    name: 'Transaction with root entrypoint',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'transaction',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
          parameters: {
            entrypoint: 'root',
            value: { string: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn' },
          },
          destination: 'KT1JHqHQdHSgWBKo6H4UfG8dw3JnZSyjGkHA',
          amount: '1000',
        },
      ],
    },
  },
  {
    name: 'Transaction with do entrypoint',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'transaction',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
          parameters: {
            entrypoint: 'do',
            value: { string: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn' },
          },
          destination: 'KT1JHqHQdHSgWBKo6H4UfG8dw3JnZSyjGkHA',
          amount: '1000',
        },
      ],
    },
  },
  {
    name: 'Transaction with do entrypoint and unit',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'transaction',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
          parameters: {
            entrypoint: 'do',
            value: { prim: 'Unit' },
          },
          destination: 'KT1JHqHQdHSgWBKo6H4UfG8dw3JnZSyjGkHA',
          amount: '1000',
        },
      ],
    },
  },
  {
    name: 'Transaction with custom entrypoint',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'transaction',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
          parameters: {
            entrypoint: 'main',
            value: {
              prim: 'Pair',
              args: [{ prim: 'Pair', args: [{ bytes: '0202' }, { int: '202' }] }, { string: 'hello' }],
            },
          },
          destination: 'KT1HPaJE1QNtuiYPgMAGhzTrs446K9wptmsR',
          amount: '1000',
        },
      ],
    },
  },
  {
    name: 'Transaction with default entrypoint and annotation',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'transaction',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
          parameters: {
            entrypoint: 'default',
            value: {
              prim: 'Pair',
              args: [{ bytes: '0202' }, { string: 'hello' }],
              annots: ['%test'],
            },
          },
          destination: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          amount: '1000',
        },
      ],
    },
  },
  // {
  //   name: 'Origination vote example',
  //   operation: {
  //     branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
  //     contents: [
  //       {
  //         kind: 'origination',
  //         counter: '1',
  //         source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
  //         fee: '10000',
  //         gas_limit: '10',
  //         storage_limit: '10',
  //         balance: '0',
  //         script: {
  //           code: voteSample,
  //           storage: voteInitSample,
  //         },
  //       },
  //     ],
  //   },
  // },
  // In `opMapping` from the file `constants.ts`, the operations before `LEVEL` are common to the protocols (delphi and edo)
  // ...extractOp(0, 117).map(op => {
  //   return {
  //     name: `Origination operation (${op})`,
  //     operation: {
  //       branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
  //       contents: [
  //         {
  //           kind: 'origination',
  //           counter: '1',
  //           source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
  //           fee: '10000',
  //           gas_limit: '10',
  //           storage_limit: '10',
  //           balance: '0',
  //           script: {
  //             code: genericCode(op),
  //             storage: genericStorage,
  //           },
  //         },
  //       ],
  //     },
  //   }
  // }),
  // {
  //   name: 'Origination with bigmap',
  //   operation: {
  //     branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
  //     contents: [
  //       {
  //         kind: 'origination',
  //         counter: '1',
  //         source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
  //         fee: '10000',
  //         gas_limit: '10',
  //         storage_limit: '10',
  //         balance: '0',
  //         script: {
  //           code: tokenBigmapCode,
  //           storage: tokenBigmapStorage,
  //         },
  //       },
  //     ],
  //   },
  // },
  // {
  //   name: 'Origination with no annot',
  //   operation: {
  //     branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
  //     contents: [
  //       {
  //         kind: 'origination',
  //         counter: '1',
  //         source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
  //         fee: '10000',
  //         gas_limit: '10',
  //         storage_limit: '10',
  //         balance: '0',
  //         script: {
  //           code: noAnnotCode,
  //           storage: noAnnotInit('tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn'),
  //         },
  //       },
  //     ],
  //   },
  // },
  {
    name: 'Multiple transaction at once',
    operation: {
      branch: 'BLzyjjHKEKMULtvkpSHxuZxx6ei6fpntH2BTkYZiLgs8zLVstvX',
      contents: [
        {
          kind: 'reveal',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          public_key: 'edpkvS5QFv7KRGfa3b87gg9DBpxSm3NpSwnjhUjNBQrRUUR66F7C9g',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
        },
        // {
        //   kind: 'origination',
        //   counter: '1',
        //   source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
        //   fee: '10000',
        //   gas_limit: '10',
        //   storage_limit: '10',
        //   balance: '0',
        //   script: {
        //     code: noAnnotCode,
        //     storage: noAnnotInit('tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn'),
        //   },
        // },
        {
          kind: 'transaction',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
          parameters: {
            entrypoint: 'default',
            value: {
              prim: 'Pair',
              args: [{ bytes: '0202' }, { string: 'hello' }],
              annots: ['%test'],
            },
          },
          destination: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          amount: '1000',
        },
        {
          kind: 'transaction',
          counter: '1',
          source: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          fee: '10000',
          gas_limit: '10',
          storage_limit: '10',
          destination: 'tz1QZ6KY7d3BuZDT1d19dUxoQrtFPN2QJ3hn',
          amount: '1000',
        },
      ],
    },
  },
]

async function forgeAndAddBlob(op: { operation: any }): Promise<{ operation: any; blob: Buffer }> {
  const forgedOp = await localForger.forge(op.operation)
  const blob = Buffer.from(forgedOp, 'hex')

  return {
    operation: op.operation,
    blob,
  }
}

export const SAMPLE_OPERATIONS = Promise.all(
  SAMPLE_OPERATIONS_NO_BLOB.map(item => {
    return forgeAndAddBlob(item)
  }),
)

export const SIMPLE_TRANSACTION = SAMPLE_OPERATIONS.then(arr => arr[5])
