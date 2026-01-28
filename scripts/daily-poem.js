#!/usr/bin/env node

/**
 * Chain Verse - Daily Poem Generator
 *
 * This script runs once per day via GitHub Actions to:
 * 1. Fetch multiple Solana blocks from the past 24 hours
 * 2. Derive keywords from block data (same logic as Rust backend)
 * 3. Generate a poem using OpenRouter AI
 * 4. Post to Bluesky
 * 5. Save poem data for the static website
 */

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');

// Poem images configuration
const POEM_IMAGES_DIR = path.join(__dirname, '..', 'poem-images');
const LAST_USED_IMAGE_FILE = path.join(POEM_IMAGES_DIR, '.last-used');

// Configuration
const SOLANA_RPC_URL = 'https://api.mainnet-beta.solana.com';
const OPENROUTER_API_URL = 'https://openrouter.ai/api/v1/chat/completions';
const BLOCKS_TO_FETCH = 12; // Fetch 12 blocks spread across 24 hours
const SLOTS_PER_SECOND = 2.5;
const CONFIRMATION_SLOTS = 32;

// Fallback models in order of preference (strongest/most reliable first)
const FALLBACK_MODELS = [
  'google/gemini-2.0-flash-001',
  'google/gemini-2.5-flash',
  'meta-llama/llama-3.1-8b-instruct',
  'mistralai/mistral-7b-instruct:free',
  'qwen/qwen-2.5-7b-instruct:free'
];

/**
 * Get a random image from poem-images folder, avoiding the last used one
 */
function getRandomImage() {
  if (!fs.existsSync(POEM_IMAGES_DIR)) {
    console.log('  No poem-images directory found');
    return null;
  }

  // Get all image files
  const imageExtensions = ['.png', '.jpg', '.jpeg', '.gif', '.webp'];
  const allImages = fs.readdirSync(POEM_IMAGES_DIR)
    .filter(file => imageExtensions.includes(path.extname(file).toLowerCase()));

  if (allImages.length === 0) {
    console.log('  No images found in poem-images/');
    return null;
  }

  // Read last used image
  let lastUsed = null;
  if (fs.existsSync(LAST_USED_IMAGE_FILE)) {
    lastUsed = fs.readFileSync(LAST_USED_IMAGE_FILE, 'utf-8').trim();
  }

  // Filter out last used (unless it's the only image)
  let availableImages = allImages.filter(img => img !== lastUsed);
  if (availableImages.length === 0) {
    availableImages = allImages; // Only one image, use it anyway
  }

  // Pick random image
  const randomIndex = Math.floor(Math.random() * availableImages.length);
  const selectedImage = availableImages[randomIndex];

  // Save as last used
  fs.writeFileSync(LAST_USED_IMAGE_FILE, selectedImage);

  console.log(`  Selected image: ${selectedImage}`);
  return path.join(POEM_IMAGES_DIR, selectedImage);
}

// Load word dictionary (BIP-39 wordlist - 2048 words)
const wordsPath = path.join(__dirname, '..', 'backend', 'words.json');
const wordDictionary = JSON.parse(fs.readFileSync(wordsPath, 'utf-8'));
const allWords = wordDictionary.words;

/**
 * Make RPC call to Solana
 */
async function rpcCall(method, params = []) {
  const response = await fetch(SOLANA_RPC_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      jsonrpc: '2.0',
      id: 1,
      method,
      params
    })
  });

  const data = await response.json();
  if (data.error) {
    throw new Error(`RPC Error: ${data.error.message}`);
  }
  return data.result;
}

/**
 * Get current slot
 */
async function getCurrentSlot() {
  return await rpcCall('getSlot', [{ commitment: 'confirmed' }]);
}

/**
 * Get block info for a specific slot
 */
async function getBlock(slot) {
  const config = {
    encoding: 'base64',
    transactionDetails: 'signatures',
    rewards: false,
    commitment: 'confirmed',
    maxSupportedTransactionVersion: 0
  };

  const block = await rpcCall('getBlock', [slot, config]);
  if (!block) {
    throw new Error(`Block not found for slot ${slot}`);
  }

  return {
    slot,
    blockhash: block.blockhash,
    previousBlockhash: block.previousBlockhash,
    blockTime: block.blockTime,
    blockHeight: block.blockHeight,
    parentSlot: block.parentSlot,
    transactionCount: block.signatures?.length || 0,
    sampleSignatures: (block.signatures || []).slice(0, 5)
  };
}

/**
 * Fetch multiple blocks spread across the past 24 hours
 */
async function fetchBlocksFromPast24Hours() {
  console.log('Fetching blocks from past 24 hours...');

  const currentSlot = await getCurrentSlot();
  const slotsIn24Hours = 24 * 60 * 60 * SLOTS_PER_SECOND; // ~216,000 slots
  const intervalSlots = Math.floor(slotsIn24Hours / BLOCKS_TO_FETCH);

  const blocks = [];

  for (let i = 0; i < BLOCKS_TO_FETCH; i++) {
    const targetSlot = currentSlot - CONFIRMATION_SLOTS - (i * intervalSlots);

    // Try to get the block, with fallback to nearby slots
    let block = null;
    for (let offset = 0; offset < 10; offset++) {
      try {
        block = await getBlock(targetSlot - offset);
        break;
      } catch (e) {
        if (offset === 9) {
          console.log(`  Skipping slot ${targetSlot} - unavailable`);
        }
      }
    }

    if (block) {
      blocks.push(block);
      console.log(`  Block ${i + 1}/${BLOCKS_TO_FETCH}: slot ${block.slot}`);
    }

    // Small delay to avoid rate limiting
    await new Promise(resolve => setTimeout(resolve, 100));
  }

  console.log(`Fetched ${blocks.length} blocks\n`);
  return blocks;
}

/**
 * Hash string to seed (same as Rust implementation)
 */
function hashToSeed(input) {
  const hash = crypto.createHash('sha256').update(input).digest();
  // Read first 8 bytes as little-endian uint64
  return hash.readBigUInt64LE(0);
}

/**
 * Derive a keyword from block data
 */
function deriveKeyword(block, source = 'blockhash') {
  let entropy;

  switch (source) {
    case 'blockhash':
      entropy = block.blockhash;
      break;
    case 'previousBlockhash':
      entropy = block.previousBlockhash;
      break;
    case 'transaction':
      entropy = block.sampleSignatures.join(':');
      break;
    default:
      entropy = block.blockhash;
  }

  const seed = hashToSeed(entropy);
  const wordIndex = Number(seed % BigInt(allWords.length));
  const word = allWords[wordIndex];

  return {
    word,
    slot: block.slot,
    blockhash: block.blockhash,
    blockTime: block.blockTime,
    source
  };
}

/**
 * Derive multiple unique keywords from blocks
 */
function deriveKeywordsFromBlocks(blocks) {
  console.log('Deriving keywords from blocks...');

  const keywords = [];
  const seenWords = new Set();

  for (const block of blocks) {
    // Try different entropy sources per block
    const sources = ['blockhash', 'previousBlockhash', 'transaction'];

    for (const source of sources) {
      try {
        const kw = deriveKeyword(block, source);
        if (!seenWords.has(kw.word)) {
          seenWords.add(kw.word);
          keywords.push(kw);
          console.log(`  "${kw.word}" from ${source} (slot ${block.slot})`);
        }
      } catch (e) {
        // Ignore errors for specific sources
      }

      // Stop if we have enough keywords
      if (keywords.length >= 16) break;
    }

    if (keywords.length >= 16) break;
  }

  console.log(`Derived ${keywords.length} unique keywords\n`);
  return keywords;
}

/**
 * Try to generate poem with a single model
 * Returns { success: boolean, poem?: string, error?: string }
 */
async function tryGenerateWithModel(apiKey, model, prompt) {
  const response = await fetch(OPENROUTER_API_URL, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${apiKey}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({
      model,
      messages: [{ role: 'user', content: prompt }]
    })
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`HTTP ${response.status}: ${errorText.substring(0, 100)}`);
  }

  const data = await response.json();
  const poem = data.choices?.[0]?.message?.content;

  if (!poem) {
    throw new Error('No poem in response');
  }

  return poem.trim();
}

/**
 * Generate poem using OpenRouter AI with fallback models
 */
async function generatePoem(keywords) {
  console.log('Generating poem with AI...');

  const apiKey = process.env.OPENROUTER_API_KEY;
  if (!apiKey) {
    throw new Error('OPENROUTER_API_KEY environment variable not set');
  }

  // Allow env override, otherwise use fallback chain
  const envModel = process.env.OPENROUTER_MODEL;
  const modelsToTry = envModel ? [envModel, ...FALLBACK_MODELS] : FALLBACK_MODELS;

  const keywordsList = keywords.map(k => k.word).join(', ');

  const prompt = `You are a poetic AI that creates beautiful, evocative poems.

Using ONLY the following keywords derived from the Solana blockchain, create a cohesive poem of 20-30 lines.

Keywords: ${keywordsList}

Instructions:
- Use all or most of these keywords naturally in the poem
- Create a coherent narrative or emotional arc
- The poem can be any mood - happy, sad, dark, light, mysterious, etc.
- Let the words guide the tone naturally
- Use vivid imagery and metaphor
- Make it flow well and feel complete
- Do NOT add a title
- Do NOT explain or comment on the poem
- ONLY output the poem itself

Write the poem now:`;

  for (const model of modelsToTry) {
    console.log(`  Trying model: ${model}`);

    try {
      const poem = await tryGenerateWithModel(apiKey, model, prompt);
      console.log(`  Success with: ${model}\n`);
      return poem;
    } catch (error) {
      console.log(`  Failed: ${error.message}`);
    }
  }

  throw new Error(`All ${modelsToTry.length} models failed to generate poem`);
}

/**
 * Split poem into chunks that fit Bluesky's 300 char limit
 */
function splitPoemIntoChunks(poem, maxChars = 290) {
  const lines = poem.split('\n');
  const chunks = [];
  let currentChunk = [];
  let currentLength = 0;

  for (const line of lines) {
    const lineLength = line.length + 1; // +1 for newline

    if (currentLength + lineLength > maxChars && currentChunk.length > 0) {
      // Save current chunk and start new one
      chunks.push(currentChunk.join('\n'));
      currentChunk = [line];
      currentLength = line.length;
    } else {
      currentChunk.push(line);
      currentLength += lineLength;
    }
  }

  // Don't forget the last chunk
  if (currentChunk.length > 0) {
    chunks.push(currentChunk.join('\n'));
  }

  return chunks;
}

/**
 * Upload image to Bluesky and return blob reference
 */
async function uploadImageToBluesky(agent, imagePath) {
  if (!imagePath || !fs.existsSync(imagePath)) {
    return null;
  }

  const imageData = fs.readFileSync(imagePath);
  const ext = path.extname(imagePath).toLowerCase();

  // Determine MIME type
  const mimeTypes = {
    '.png': 'image/png',
    '.jpg': 'image/jpeg',
    '.jpeg': 'image/jpeg',
    '.gif': 'image/gif',
    '.webp': 'image/webp'
  };
  const mimeType = mimeTypes[ext] || 'image/png';

  const response = await agent.uploadBlob(imageData, { encoding: mimeType });
  return response.data.blob;
}

/**
 * Post to Bluesky as a thread
 */
async function postToBluesky(poem, keywords) {
  console.log('Posting to Bluesky...');

  const handle = process.env.BLUESKY_HANDLE;
  const appPassword = process.env.BLUESKY_APP_PASSWORD;

  if (!handle || !appPassword) {
    console.log('  Bluesky credentials not set, skipping...\n');
    return null;
  }

  const { BskyAgent } = await import('@atproto/api');

  const agent = new BskyAgent({
    service: 'https://bsky.social'
  });

  try {
    // Login to Bluesky
    await agent.login({
      identifier: handle,
      password: appPassword
    });

    const websiteUrl = process.env.WEBSITE_URL || '';

    // Get random image for the first post
    console.log('  Selecting image for post...');
    const imagePath = getRandomImage();
    let imageBlob = null;
    if (imagePath) {
      console.log('  Uploading image to Bluesky...');
      imageBlob = await uploadImageToBluesky(agent, imagePath);
      if (imageBlob) {
        console.log('  Image uploaded successfully');
      }
    }

    // Split poem into chunks for thread
    const chunks = splitPoemIntoChunks(poem);
    console.log(`  Splitting poem into ${chunks.length} posts...`);

    let rootUri = null;
    let rootCid = null;
    let parentUri = null;
    let parentCid = null;

    for (let i = 0; i < chunks.length; i++) {
      let text = chunks[i];

      // Add thread indicator to first post
      if (i === 0 && chunks.length > 1) {
        text = `${text}\n\n🧵 1/${chunks.length}`;
      } else if (chunks.length > 1) {
        text = `${text}\n\n${i + 1}/${chunks.length}`;
      }

      // Add footer to last post
      if (i === chunks.length - 1) {
        text = `${text}\n\n🔗 From today's Solana blocks`;
        if (websiteUrl) {
          text += `\n${websiteUrl}`;
        }
      }

      // Build post record
      const postRecord = {
        text,
        createdAt: new Date().toISOString()
      };

      // Add image embed to first post only
      if (i === 0 && imageBlob) {
        postRecord.embed = {
          $type: 'app.bsky.embed.images',
          images: [{
            alt: 'Chain Verse - Daily poem inspired by Solana blockchain',
            image: imageBlob
          }]
        };
      }

      // Add reply reference for thread continuation
      if (parentUri && parentCid) {
        postRecord.reply = {
          root: { uri: rootUri, cid: rootCid },
          parent: { uri: parentUri, cid: parentCid }
        };
      }

      const result = await agent.post(postRecord);
      console.log(`  Posted ${i + 1}/${chunks.length}: ${result.uri}`);

      // Store references for threading
      if (i === 0) {
        rootUri = result.uri;
        rootCid = result.cid;
      }
      parentUri = result.uri;
      parentCid = result.cid;

      // Small delay between posts
      if (i < chunks.length - 1) {
        await new Promise(resolve => setTimeout(resolve, 500));
      }
    }

    console.log(`  Thread complete!\n`);
    return rootUri;
  } catch (error) {
    console.log(`  Bluesky error: ${error.message}\n`);
    return null;
  }
}

/**
 * Save poem data for the static website
 */
function savePoemData(poem, keywords, postUri) {
  console.log('Saving poem data...');

  const today = new Date().toISOString().split('T')[0];
  const dataDir = path.join(__dirname, '..', 'frontend', 'public', 'data');

  // Ensure data directory exists
  if (!fs.existsSync(dataDir)) {
    fs.mkdirSync(dataDir, { recursive: true });
  }

  // Today's poem data
  const todayData = {
    date: today,
    poem: {
      content: poem,
      generatedAt: new Date().toISOString()
    },
    keywords: keywords.map(k => ({
      word: k.word,
      slot: k.slot,
      source: k.source
    })),
    blueskyUri: postUri,
    poemReady: true,
    keywordsCollected: keywords.length,
    keywordsNeeded: 8
  };

  // Save today's data
  const todayPath = path.join(dataDir, 'today.json');
  fs.writeFileSync(todayPath, JSON.stringify(todayData, null, 2));
  console.log(`  Saved: ${todayPath}`);

  // Load or create archive
  const archivePath = path.join(dataDir, 'archive.json');
  let archive = [];
  if (fs.existsSync(archivePath)) {
    archive = JSON.parse(fs.readFileSync(archivePath, 'utf-8'));
  }

  // Add today's poem to archive (avoid duplicates)
  const existingIndex = archive.findIndex(p => p.date === today);
  if (existingIndex >= 0) {
    archive[existingIndex] = todayData;
  } else {
    archive.unshift(todayData);
  }

  // Keep last 30 days
  archive = archive.slice(0, 30);

  fs.writeFileSync(archivePath, JSON.stringify(archive, null, 2));
  console.log(`  Saved: ${archivePath}`);
  console.log('Done!\n');
}

/**
 * Main function
 */
async function main() {
  console.log('🔗 Chain Verse - Daily Poem Generator\n');
  console.log(`Date: ${new Date().toISOString()}\n`);

  try {
    // 1. Fetch blocks
    const blocks = await fetchBlocksFromPast24Hours();

    if (blocks.length === 0) {
      throw new Error('No blocks fetched');
    }

    // 2. Derive keywords
    const keywords = deriveKeywordsFromBlocks(blocks);

    if (keywords.length < 8) {
      throw new Error(`Not enough keywords: ${keywords.length}`);
    }

    // 3. Generate poem
    const poem = await generatePoem(keywords);

    console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
    console.log('TODAY\'S POEM:');
    console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
    console.log(poem);
    console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');

    // 4. Post to Bluesky
    const postUri = await postToBluesky(poem, keywords);

    // 5. Save data
    savePoemData(poem, keywords, postUri);

    console.log('✅ Daily poem generation complete!');

  } catch (error) {
    console.error('❌ Error:', error.message);
    process.exit(1);
  }
}

main();
