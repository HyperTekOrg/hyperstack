import { HyperStack } from 'hyperstack-typescript';
import { OREROUND_STACK, type OreRound } from 'hyperstack-stacks/ore';

// const URL = 'wss://ore.stack.usehyperstack.com';
const URL = 'ws://localhost:8878';

function printRound(round: OreRound) {
  console.log(`\n=== Round #${round.id?.round_id ?? 'N/A'} ===`);
  console.log(`Address: ${round.id?.round_address ?? 'N/A'}`);
  console.log(`Motherlode: ${round.state?.motherlode ?? 'N/A'}`);
  console.log(`Total Deployed: ${round.state?.total_deployed ?? 'N/A'}`);
  console.log(`Expires At: ${round.state?.expires_at ?? 'N/A'}`);
  console.log(`Deploy Count: ${round.metrics?.deploy_count ?? 0}`);
  console.log();
}

async function main() {
  const hs = await HyperStack.connect(URL, {
    stack: OREROUND_STACK,
  });
  console.log(`Connected to ${URL}\n`);

  for await (const round of hs.views.OreRound.latest.use({ take: 1 })) {
    printRound(round);
  }
}

main().catch((err) => {
  console.error('Error:', err);
  process.exit(1);
});
