import { HyperStack } from 'hyperstack-typescript';
import {
  ORE_STREAM_STACK,
  type OreRound,
  type OreTreasury,
  type OreMiner,
} from 'hyperstack-stacks/ore';

function printRound(round: OreRound) {
  console.log(`\n=== Round #${round.id?.round_id ?? 'N/A'} ===`);
  console.log(`Address: ${round.id?.round_address ?? 'N/A'}`);
  console.log(`Motherlode: ${round.state?.motherlode ?? 'N/A'}`);
  console.log(`Total Deployed: ${round.state?.total_deployed ?? 'N/A'}`);
  console.log(`Expires At: ${round.state?.expires_at ?? 'N/A'}`);
  console.log(`Deploy Count: ${round.metrics?.deploy_count ?? 0}`);
  console.log();
}

function printTreasury(treasury: OreTreasury) {
  console.log(`\n=== Treasury ===`);
  console.log(`Address: ${treasury.id?.address ?? 'N/A'}`);
  console.log(`Balance: ${treasury.state?.balance ?? 'N/A'}`);
  console.log(`Motherlode: ${treasury.state?.motherlode ?? 'N/A'}`);
  console.log(`Total Refined: ${treasury.state?.total_refined ?? 'N/A'}`);
  console.log(`Total Staked: ${treasury.state?.total_staked ?? 'N/A'}`);
  console.log(`Total Unclaimed: ${treasury.state?.total_unclaimed ?? 'N/A'}`);
  console.log();
}

function printMiner(miner: OreMiner) {
  console.log(`\n=== Miner ===`);
  console.log(`Authority: ${miner.id?.authority ?? 'N/A'}`);
  console.log(`Miner Address: ${miner.id?.miner_address ?? 'N/A'}`);
  console.log(`Round ID: ${miner.state?.round_id ?? 'N/A'}`);
  console.log(`Rewards SOL: ${miner.rewards?.rewards_sol ?? 'N/A'}`);
  console.log(`Rewards ORE: ${miner.rewards?.rewards_ore ?? 'N/A'}`);
  console.log(`Lifetime Deployed: ${miner.rewards?.lifetime_deployed ?? 'N/A'}`);
  console.log(`Automation Executor: ${miner.automation?.executor ?? 'N/A'}`);
  console.log();
}

async function main() {
  const hs = await HyperStack.connect(ORE_STREAM_STACK);

  const treasuryAddress = "45db2FSR4mcXdSVVZbKbwojU6uYDpMyhpEi7cC8nHaWG";
  const minerAuthority = "Fm9wsyJf5HqAAyTyfSVWxpYpHTwhTZoNvMbfy5oCqZhs";

  if (!treasuryAddress || !minerAuthority) {
    console.error(
      'Set ORE_TREASURY_ADDRESS and ORE_MINER_AUTHORITY env vars to the on-chain account pubkeys.'
    );
    process.exit(1);
  }

  // --- OreRound: latest round from the list view ---
  console.log('--- Listening for latest OreRound ---');
  for await (const round of hs.views.OreRound.latest.use({ take: 1 })) {
    printRound(round);
    break;
  }

  // --- OreTreasury: singleton state lookup by treasury address ---
  console.log('--- Fetching OreTreasury state ---');
  for await (const treasury of hs.views.OreTreasury.state.use(treasuryAddress, { take: 1 })) {
    printTreasury(treasury);
    break;
  }

  // --- OreMiner: state lookup by miner authority pubkey ---
  console.log('--- Fetching OreMiner state ---');
  for await (const miner of hs.views.OreMiner.state.use(minerAuthority, { take: 1 })) {
    printMiner(miner);
    break;
  }
}

main().catch((err) => {
  console.error('Error:', err);
  process.exit(1);
});
