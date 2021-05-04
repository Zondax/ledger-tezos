///*******************************************************************************
//*   (c) 2018, 2019 Zondax GmbH
//*   (c) 2016 Ledger
//*
//*  Licensed under the Apache License, Version 2.0 (the "License");
//*  you may not use this file except in compliance with the License.
//*  You may obtain a copy of the License at
//*
//*      http://www.apache.org/licenses/LICENSE-2.0
//*
//*  Unless required by applicable law or agreed to in writing, software
//*  distributed under the License is distributed on an "AS IS" BASIS,
//*  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//*  See the License for the specific language governing permissions and
//*  limitations under the License.
//********************************************************************************/
//
//#include "app_main.h"
//
//#include <string.h>
//#include <os_io_seproxyhal.h>
//#include <os.h>
//
//#include "view.h"
//#include "actions.h"
//#include "tx.h"
//#include "addr.h"
//#include "crypto.h"
//#include "coin.h"
//#include "zxmacros.h"
//
//__Z_INLINE void handleGetAddrSecp256K1(volatile uint32_t *flags, volatile uint32_t *tx, uint32_t rx) {
//    extractHDPath(rx, OFFSET_DATA);
//
//    uint8_t requireConfirmation = G_io_apdu_buffer[OFFSET_P1];
//    uint8_t network = G_io_apdu_buffer[OFFSET_P2];
//
//    if (requireConfirmation) {
//        app_fill_address(addr_secp256k1);
//
//        view_review_init(addr_getItem, addr_getNumItems, app_reply_address);
//        view_review_show();
//
//        *flags |= IO_ASYNCH_REPLY;
//        return;
//    }
//
//    *tx = app_fill_address(addr_secp256k1);
//    THROW(APDU_CODE_OK);
//}
//
//__Z_INLINE void handleSignSecp256K1(volatile uint32_t *flags, volatile uint32_t *tx, uint32_t rx) {
//    if (!process_chunk(tx, rx)) {
//        THROW(APDU_CODE_OK);
//    }
//
//    const char *error_msg = tx_parse();
//
//    if (error_msg != NULL) {
//        int error_msg_length = strlen(error_msg);
//        MEMCPY(G_io_apdu_buffer, error_msg, error_msg_length);
//        *tx += (error_msg_length);
//        THROW(APDU_CODE_DATA_INVALID);
//    }
//
//    zemu_log_stack("tx_parse done\n");
//
//    CHECK_CANARY()
//    view_review_init(tx_getItem, tx_getNumItems, app_sign);
//    view_review_show();
//    *flags |= IO_ASYNCH_REPLY;
//}
