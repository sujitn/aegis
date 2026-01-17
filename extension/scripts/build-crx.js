#!/usr/bin/env node
/**
 * Build CRX package for the Aegis browser extension.
 *
 * Usage:
 *   npm run build:crx
 *   node scripts/build-crx.js [--key=path/to/key.pem] [--output=path/to/output.crx]
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

// Parse command line arguments
const args = process.argv.slice(2).reduce((acc, arg) => {
  const [key, value] = arg.replace(/^--/, '').split('=');
  acc[key] = value || true;
  return acc;
}, {});

const ROOT_DIR = path.resolve(__dirname, '..');
const KEY_PATH = args.key || path.join(ROOT_DIR, 'key.pem');
const RELEASE_DIR = path.join(ROOT_DIR, 'release');

// Read manifest to get version
const manifest = JSON.parse(fs.readFileSync(path.join(ROOT_DIR, 'manifest.json'), 'utf8'));
const version = manifest.version || '1.0.0';
const OUTPUT_PATH = args.output || path.join(RELEASE_DIR, `aegis-extension-${version}.crx`);

/**
 * Generate a new RSA key pair for signing.
 */
function generateKey() {
  console.log('Generating new signing key...');
  const { privateKey } = crypto.generateKeyPairSync('rsa', {
    modulusLength: 2048,
    publicKeyEncoding: { type: 'spki', format: 'pem' },
    privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
  });
  return privateKey;
}

/**
 * Get or create the signing key.
 */
function getSigningKey() {
  if (fs.existsSync(KEY_PATH)) {
    console.log(`Using existing key: ${KEY_PATH}`);
    return fs.readFileSync(KEY_PATH, 'utf8');
  }

  const key = generateKey();
  fs.writeFileSync(KEY_PATH, key, { mode: 0o600 });
  console.log(`Generated new key: ${KEY_PATH}`);
  return key;
}

/**
 * Collect all files to include in the CRX.
 */
function collectFiles(dir, baseDir = dir) {
  const files = [];
  const entries = fs.readdirSync(dir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    const relativePath = path.relative(baseDir, fullPath);

    // Skip certain files/directories
    if (entry.name.startsWith('.') ||
        entry.name === 'node_modules' ||
        entry.name === 'src' ||
        entry.name === 'scripts' ||
        entry.name === 'release' ||
        entry.name === 'key.pem' ||
        entry.name === 'tsconfig.json' ||
        entry.name === 'package.json' ||
        entry.name === 'package-lock.json') {
      continue;
    }

    if (entry.isDirectory()) {
      files.push(...collectFiles(fullPath, baseDir));
    } else {
      files.push({ path: relativePath, content: fs.readFileSync(fullPath) });
    }
  }

  return files;
}

/**
 * Create a ZIP archive of the extension files.
 */
function createZipBuffer(files) {
  // Simple ZIP implementation for CRX
  // CRX format needs a ZIP file as the payload

  const parts = [];
  const centralDirectory = [];
  let offset = 0;

  for (const file of files) {
    const nameBuffer = Buffer.from(file.path.replace(/\\/g, '/'), 'utf8');
    const content = file.content;

    // Local file header
    const localHeader = Buffer.alloc(30 + nameBuffer.length);
    localHeader.writeUInt32LE(0x04034b50, 0); // Local file header signature
    localHeader.writeUInt16LE(20, 4); // Version needed to extract
    localHeader.writeUInt16LE(0, 6); // General purpose bit flag
    localHeader.writeUInt16LE(0, 8); // Compression method (stored)
    localHeader.writeUInt16LE(0, 10); // Last mod file time
    localHeader.writeUInt16LE(0, 12); // Last mod file date

    // CRC-32
    const crc = crc32(content);
    localHeader.writeUInt32LE(crc, 14);
    localHeader.writeUInt32LE(content.length, 18); // Compressed size
    localHeader.writeUInt32LE(content.length, 22); // Uncompressed size
    localHeader.writeUInt16LE(nameBuffer.length, 26); // File name length
    localHeader.writeUInt16LE(0, 28); // Extra field length
    nameBuffer.copy(localHeader, 30);

    parts.push(localHeader, content);

    // Central directory entry
    const centralEntry = Buffer.alloc(46 + nameBuffer.length);
    centralEntry.writeUInt32LE(0x02014b50, 0); // Central directory signature
    centralEntry.writeUInt16LE(20, 4); // Version made by
    centralEntry.writeUInt16LE(20, 6); // Version needed to extract
    centralEntry.writeUInt16LE(0, 8); // General purpose bit flag
    centralEntry.writeUInt16LE(0, 10); // Compression method
    centralEntry.writeUInt16LE(0, 12); // Last mod file time
    centralEntry.writeUInt16LE(0, 14); // Last mod file date
    centralEntry.writeUInt32LE(crc, 16); // CRC-32
    centralEntry.writeUInt32LE(content.length, 20); // Compressed size
    centralEntry.writeUInt32LE(content.length, 24); // Uncompressed size
    centralEntry.writeUInt16LE(nameBuffer.length, 28); // File name length
    centralEntry.writeUInt16LE(0, 30); // Extra field length
    centralEntry.writeUInt16LE(0, 32); // File comment length
    centralEntry.writeUInt16LE(0, 34); // Disk number start
    centralEntry.writeUInt16LE(0, 36); // Internal file attributes
    centralEntry.writeUInt32LE(0, 38); // External file attributes
    centralEntry.writeUInt32LE(offset, 42); // Relative offset of local header
    nameBuffer.copy(centralEntry, 46);

    centralDirectory.push(centralEntry);
    offset += localHeader.length + content.length;
  }

  const centralDirOffset = offset;
  const centralDirSize = centralDirectory.reduce((sum, buf) => sum + buf.length, 0);

  // End of central directory
  const endOfCentralDir = Buffer.alloc(22);
  endOfCentralDir.writeUInt32LE(0x06054b50, 0); // End of central directory signature
  endOfCentralDir.writeUInt16LE(0, 4); // Number of this disk
  endOfCentralDir.writeUInt16LE(0, 6); // Disk where central directory starts
  endOfCentralDir.writeUInt16LE(files.length, 8); // Number of central directory records on this disk
  endOfCentralDir.writeUInt16LE(files.length, 10); // Total number of central directory records
  endOfCentralDir.writeUInt32LE(centralDirSize, 12); // Size of central directory
  endOfCentralDir.writeUInt32LE(centralDirOffset, 16); // Offset of start of central directory
  endOfCentralDir.writeUInt16LE(0, 20); // Comment length

  return Buffer.concat([...parts, ...centralDirectory, endOfCentralDir]);
}

/**
 * CRC-32 calculation.
 */
function crc32(buffer) {
  let crc = 0xffffffff;
  const table = getCrc32Table();

  for (let i = 0; i < buffer.length; i++) {
    crc = (crc >>> 8) ^ table[(crc ^ buffer[i]) & 0xff];
  }

  return (crc ^ 0xffffffff) >>> 0;
}

let crc32Table = null;
function getCrc32Table() {
  if (crc32Table) return crc32Table;

  crc32Table = new Uint32Array(256);
  for (let i = 0; i < 256; i++) {
    let c = i;
    for (let j = 0; j < 8; j++) {
      c = (c & 1) ? (0xedb88320 ^ (c >>> 1)) : (c >>> 1);
    }
    crc32Table[i] = c >>> 0;
  }
  return crc32Table;
}

/**
 * Create CRX3 format package.
 */
function createCrx3(zipBuffer, privateKeyPem) {
  // CRX3 format:
  // - Magic number: "Cr24" (4 bytes)
  // - Version: 3 (4 bytes, little-endian)
  // - Header length (4 bytes, little-endian)
  // - Header (protocol buffer)
  // - ZIP archive

  const privateKey = crypto.createPrivateKey(privateKeyPem);
  const publicKey = crypto.createPublicKey(privateKey);
  const publicKeyDer = publicKey.export({ type: 'spki', format: 'der' });

  // Sign the data: "CRX3 SignedData\x00" + header_size (4 bytes) + signed_header_data + ZIP
  // For simplicity, we'll create a basic CRX3 structure

  // Create signed data
  const signedData = Buffer.concat([
    Buffer.from('CRX3 SignedData\x00'),
    zipBuffer
  ]);

  // Sign with SHA256
  const sign = crypto.createSign('SHA256');
  sign.update(signedData);
  const signature = sign.sign(privateKey);

  // Build CRX3 header (simplified protobuf)
  // Field 2: sha256_with_rsa proof
  //   Field 1: public_key
  //   Field 2: signature

  function encodeVarint(value) {
    const bytes = [];
    while (value > 127) {
      bytes.push((value & 0x7f) | 0x80);
      value >>>= 7;
    }
    bytes.push(value);
    return Buffer.from(bytes);
  }

  function encodeField(fieldNum, wireType, data) {
    const tag = (fieldNum << 3) | wireType;
    if (wireType === 2) { // Length-delimited
      return Buffer.concat([encodeVarint(tag), encodeVarint(data.length), data]);
    }
    return Buffer.concat([encodeVarint(tag), data]);
  }

  // Build proof (field 2 in CrxFileHeader)
  const proof = Buffer.concat([
    encodeField(1, 2, publicKeyDer), // public_key
    encodeField(2, 2, signature), // signature
  ]);

  // Build header
  const header = encodeField(2, 2, proof); // sha256_with_rsa

  // Build CRX3 file
  const magic = Buffer.from('Cr24');
  const version = Buffer.alloc(4);
  version.writeUInt32LE(3, 0);
  const headerLength = Buffer.alloc(4);
  headerLength.writeUInt32LE(header.length, 0);

  return Buffer.concat([magic, version, headerLength, header, zipBuffer]);
}

/**
 * Main function.
 */
async function main() {
  console.log('Building Aegis CRX package...');
  console.log(`Extension version: ${version}`);

  // Ensure release directory exists
  if (!fs.existsSync(RELEASE_DIR)) {
    fs.mkdirSync(RELEASE_DIR, { recursive: true });
  }

  // Check that dist folder exists (extension was built)
  const distPath = path.join(ROOT_DIR, 'dist');
  if (!fs.existsSync(distPath)) {
    console.error('Error: dist/ folder not found. Run "npm run build" first.');
    process.exit(1);
  }

  // Get signing key
  const privateKey = getSigningKey();

  // Collect files
  console.log('Collecting extension files...');
  const files = collectFiles(ROOT_DIR);
  console.log(`Found ${files.length} files`);

  // Create ZIP
  console.log('Creating ZIP archive...');
  const zipBuffer = createZipBuffer(files);

  // Create CRX3
  console.log('Creating CRX3 package...');
  const crxBuffer = createCrx3(zipBuffer, privateKey);

  // Write output
  fs.writeFileSync(OUTPUT_PATH, crxBuffer);
  console.log(`CRX package created: ${OUTPUT_PATH}`);
  console.log(`Size: ${(crxBuffer.length / 1024).toFixed(2)} KB`);

  // Also create a ZIP for store submission
  const zipPath = path.join(RELEASE_DIR, `aegis-extension-${version}.zip`);
  fs.writeFileSync(zipPath, zipBuffer);
  console.log(`ZIP package created: ${zipPath}`);
}

main().catch(err => {
  console.error('Error:', err.message);
  process.exit(1);
});
