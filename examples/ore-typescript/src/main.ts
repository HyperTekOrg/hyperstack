import { HyperStack, type Update } from 'hyperstack-typescript';
import { OREROUND_STACK, type OreRound } from 'hyperstack-stacks/ore';

const URL = 'wss://ore.stack.usehyperstack.com';

function printRoundUpdate(prefix: string, key: string, data: OreRound) {
  console.log(`\n[${prefix}] === Round Update ===`);
  console.log(`[${prefix}] Key: ${key}`);
  console.log(`[${prefix}] Round ID: ${data.id?.round_id ?? 'N/A'}`);
  console.log(`[${prefix}] Address: ${data.id?.round_address ?? 'N/A'}`);
  console.log(`[${prefix}] Motherlode: ${data.state?.motherlode ?? 'N/A'}`);
  console.log(`[${prefix}] Total Deployed: ${data.state?.total_deployed ?? 'N/A'}`);
  console.log(`[${prefix}] Expires At: ${data.state?.expires_at ?? 'N/A'}`);
  console.log(`[${prefix}] Metrics: ${JSON.stringify(data.metrics)}`);
  console.log();
}

function handleUpdate(prefix: string, update: Update<OreRound>) {
  if (update.type === 'upsert' || update.type === 'patch') {
    printRoundUpdate(prefix, update.key, update.data as OreRound);
  } else if (update.type === 'delete') {
    console.log(`[${prefix}] Round deleted: ${update.key}\n`);
  }
}

async function main() {
  const hs = await HyperStack.connect(URL, {
    stack: OREROUND_STACK,
  });
  console.log(`Connected to ${URL}`);

  console.log('=== Watching Latest view ===');

  for await (const update of hs.views.OreRound.latest.watch()) {
    handleUpdate('LATEST', update);
  }
}

main().catch((err) => {
  console.error('Error:', err);
  process.exit(1);
});
