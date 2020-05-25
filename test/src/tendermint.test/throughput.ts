// Copyright 2020 Kodebox, Inc.
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
(async () => {
    const server = process.env.SERVER!;
    const sdk = new SDK({
        server,
        networkId: "bc"
    });

    const tempAccount = process.env.RICH_ACCOUNT!;
    const tempPrivate = process.env.RICH_SECRET!;

    const beagleStakeHolder1 = "bccqxkppqfqwwl6vwge62qq22eh3xkmzqwvschr8thm";
    const baseSeq = await sdk.rpc.chain.getSeq(tempAccount);
    console.log("base seq fetched");

    const bundleSize = 2000;
    const transactions = [];
    for (let i = 0; i < bundleSize; i++) {
        const recipient = beagleStakeHolder1;
        const transaction = sdk.core
        .createPayTransaction({
            recipient,
            quantity: 1
        })
        .sign({
            secret: tempPrivate,
            seq: baseSeq + i,
            fee: 100
        });
        transactions.push(transaction);
        console.log(`${i}th transaction has generated`);
    }
    const reversed = transactions.reverse();

    const startTime = new Date();
    console.log(`Start at: ${startTime}`);

    Promise.all(reversed.map(async (transaction, i) => {
        await sdk.rpc.chain.sendSignedTransaction(transaction);
    })).then(() => {
        const endTime = new Date();
        console.log(`End at: ${endTime}`);
        const throughput =
            (bundleSize * 1000.0) / (endTime.getTime() - startTime.getTime());
        console.log(
            `Elapsed time (ms): ${endTime.getTime() - startTime.getTime()}`
        );
        console.log(`throughput: ${throughput}`);
    }).catch(err => console.log(err));
})().catch(console.error);
