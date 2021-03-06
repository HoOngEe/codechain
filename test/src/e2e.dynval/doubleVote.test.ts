// Copyright 2019 Kodebox, Inc.
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

import * as chai from "chai";
import { expect } from "chai";
import * as chaiAsPromised from "chai-as-promised";
import * as stake from "codechain-stakeholder-sdk";
import "mocha";

import { Mock } from "../helper/mock";
import { Step as TendermintStep } from "../helper/mock/tendermintMessage";
import { validators } from "../../tendermint.dynval/constants";
import { PromiseExpect } from "../helper/promise";
import { fullyConnect, setTermTestTimeout, withNodes } from "./setup";

chai.use(chaiAsPromised);

describe("Double vote detection", function() {
    const promiseExpect = new PromiseExpect();

    const [alice, betty, charlie] = validators;

    const { nodes } = withNodes(this, {
        promiseExpect,
        validators: [
            { signer: alice, delegation: 5000, deposit: 10_000_000 - 0 },
            { signer: betty, delegation: 5000, deposit: 10_000_000 - 1 },
            { signer: charlie, delegation: 5000, deposit: 10_000_000 - 2 }
        ],
        overrideParams: {
            termSeconds: 20,
            minNumOfValidators: 3
        }
    });

    let mock: Mock;

    beforeEach(async function() {
        const aliceNode = nodes[0];
        mock = new Mock("0.0.0.0", aliceNode.port, aliceNode.sdk.networkId);
    });

    it("Should report if double vote for prevote is detected", async function() {
        const termWaiter = setTermTestTimeout(this, { terms: 1 });

        const aliceNode = nodes[0];
        const bettyNode = nodes[1];

        // Kill betty and start sending double votes for all the votes
        await bettyNode.clean();
        await mock.establishWithoutSync();
        mock.startDoubleVote(alice.privateKey, TendermintStep.Prevote);
        await termWaiter.waitForTermPeriods(0.5, 0);

        // Revive betty and check if alice is banned
        await bettyNode.start();
        await fullyConnect(nodes, promiseExpect);
        await termWaiter.waitNodeUntilTerm(aliceNode, {
            target: 2,
            termPeriods: 1
        });
        const banned = await stake.getBanned(aliceNode.sdk);
        expect(banned.map(b => b.toString())).to.include(
            alice.platformAddress.toString()
        );
    });

    it("Should report if double vote for precommit is detected", async function() {
        const termWaiter = setTermTestTimeout(this, { terms: 1 });

        const aliceNode = nodes[0];
        const bettyNode = nodes[1];

        // Kill betty and start sending double votes for all the votes
        await bettyNode.clean();
        await mock.establishWithoutSync();
        mock.startDoubleVote(alice.privateKey, TendermintStep.Precommit);
        await termWaiter.waitForTermPeriods(0.5, 0);

        // Revive betty and check if alice is banned
        await bettyNode.start();
        await fullyConnect(nodes, promiseExpect);
        await termWaiter.waitNodeUntilTerm(aliceNode, {
            target: 2,
            termPeriods: 1
        });
        const banned = await stake.getBanned(aliceNode.sdk);
        expect(banned.map(b => b.toString())).to.include(
            alice.platformAddress.toString()
        );
    });

    afterEach(async function() {
        await mock.end();
        promiseExpect.checkFulfilled();
    });
});
