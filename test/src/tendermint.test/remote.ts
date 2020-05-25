#!/usr/bin/env -S npx ts-node

// Copyright 2018-2019 Kodebox, Inc.
// This file is part of CodeChain.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

import { SDK } from "codechain-sdk";
import { H256 } from "codechain-sdk/lib/core/classes";
import { wait } from "../helper/promise";
import { makeRandomH256 } from "../helper/random";

(async () => {
    const numTransactions = parseInt(process.env.TEST_NUM_TXS || "10000", 10);

    const sdk = new SDK({
        server: "http://192.168.1.101:2487",
        networkId: "bc"
    });

    const tempAccount = "bccqxsd0f56vwydcndezvhhc5klgwj4yrle4s22j075";
    const tempPrivate =
        "a056c66080e627a0fa32c0c9fa898d6c074f1af4d896f0a16cee27cb7e129a8b";

    const beagleStakeHolder1 = "bccqy204w0m6stuahxlx3p58kc0hsgd42npqcrx8lce";
    const beagleStakeHolder1Password = "ZjXBREPbc9mcZRyY";
    const transactions = [];
    const baseSeq = await sdk.rpc.chain.getSeq(tempAccount);
    console.log("Seq fetch has been done");

    for (let i = 0; i < numTransactions; i++) {
        const recipient = beagleStakeHolder1;
        const transaciton = sdk.core
            .createPayTransaction({
                recipient,
                quantity: 1
            })
            .sign({
                secret: tempPrivate,
                seq: baseSeq + i,
                fee: 100
            });
        transactions.push(transaciton);
        console.log(`${i}th transaction is generated`);
    }

    let lastHash: H256 = H256.zero();
    for (let i = numTransactions - 1; i > 0; i--) {
        lastHash = await sdk.rpc.chain.sendSignedTransaction(transactions[i]);
        console.log(`${i}th transcation is sent`);
    }
    const startTime = new Date();
    console.log(`Start at: ${startTime}`);
    await sdk.rpc.chain.sendSignedTransaction(transactions[0]);

    while (true) {
        const result = await sdk.rpc.chain.containsTransaction(lastHash);
        console.log(`Node result: ${result}`);
        if (result) {
            break;
        }

        await wait(500);
    }
    const endTime = new Date();
    console.log(`End at: ${endTime}`);
    const tps =
        (numTransactions * 1000.0) / (endTime.getTime() - startTime.getTime());
    console.log(
        `Elapsed time (ms): ${endTime.getTime() - startTime.getTime()}`
    );
    console.log(`TPS: ${tps}`);
})().catch(console.error);
