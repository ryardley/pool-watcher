// Browser-side sealed-box for the viewing key. Seals the pasted plaintext VK
// to the scanner's X25519 public key so the app only ever relays ciphertext.
//
// Envelope (base64): ephemeralPublicKey(32) || nonce(24) || ciphertext(N+16)
//
//   shared = x25519(ephPriv, recipientPub)
//   key    = hkdf(sha256, shared, salt="", info="railgun-scanner-vk", 32)
//   ct     = xchacha20poly1305(key, nonce).encrypt(utf8(plaintext))
//
// Mirrors `docs/app/spec.md` / the scanner's `sealedbox` KeyService.

import { x25519 } from '@noble/curves/ed25519';
import { hkdf } from '@noble/hashes/hkdf';
import { sha256 } from '@noble/hashes/sha256';
import { xchacha20poly1305 } from '@noble/ciphers/chacha';

const HKDF_INFO = new TextEncoder().encode('railgun-scanner-vk');
const NONCE_LEN = 24;
const PUB_LEN = 32;

function bytesToBase64(bytes: Uint8Array): string {
  let binary = '';
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary);
}

function base64ToBytes(b64: string): Uint8Array {
  const binary = atob(b64.trim());
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

function randomBytes(len: number): Uint8Array {
  const bytes = new Uint8Array(len);
  crypto.getRandomValues(bytes);
  return bytes;
}

/** Seal `plaintext` (the viewing-key hex string) to `recipientPubB64`. */
export function seal(plaintext: string, recipientPubB64: string): string {
  const recipientPub = base64ToBytes(recipientPubB64);
  const ephPriv = x25519.utils.randomPrivateKey();
  const ephPub = x25519.getPublicKey(ephPriv);
  const shared = x25519.getSharedSecret(ephPriv, recipientPub);
  const key = hkdf(sha256, shared, new Uint8Array(0), HKDF_INFO, 32);
  const nonce = randomBytes(NONCE_LEN);
  const ciphertext = xchacha20poly1305(key, nonce).encrypt(
    new TextEncoder().encode(plaintext),
  );
  const envelope = new Uint8Array(PUB_LEN + NONCE_LEN + ciphertext.length);
  envelope.set(ephPub, 0);
  envelope.set(nonce, PUB_LEN);
  envelope.set(ciphertext, PUB_LEN + NONCE_LEN);
  return bytesToBase64(envelope);
}

function setStatus(text: string, className: string): void {
  const el = document.getElementById('status');
  if (!el) return;
  el.textContent = text;
  el.className = className;
}

async function refreshStatus(): Promise<void> {
  try {
    const resp = await fetch('/api/status');
    const data = await resp.json();
    if (data.ready) {
      setStatus(`Scanner ready.\nWallet: ${data.walletAddress}`, 'ready');
    } else {
      setStatus('Scanner not initialized. Paste a viewing key below.', 'waiting');
    }
  } catch {
    setStatus('Could not reach the app API.', 'error');
  }
}

async function init(): Promise<void> {
  const form = document.getElementById('init-form') as HTMLFormElement | null;
  const input = document.getElementById('viewing-key') as HTMLTextAreaElement | null;
  if (!form || !input) return;

  let pubkey = '';
  try {
    const resp = await fetch('/api/scanner-pubkey');
    const data = await resp.json();
    pubkey = data.publicKey;
    setStatus('Ready. Paste a viewing key to initialize.', 'waiting');
  } catch {
    setStatus('Could not fetch the scanner public key.', 'error');
  }

  form.addEventListener('submit', (event) => {
    void (async () => {
      event.preventDefault();
      const vk = input.value.trim();
      if (!vk || !pubkey) return;
      setStatus('Sealing viewing key…', 'waiting');
      let keyCiphertext: string;
      try {
        keyCiphertext = seal(vk, pubkey);
      } catch (err) {
        setStatus(`Seal failed: ${String(err)}`, 'error');
        return;
      }
      try {
        const resp = await fetch('/api/init', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ keyCiphertext }),
        });
        const data = await resp.json();
        if (resp.ok) {
          setStatus(`Scanner ready.\nWallet: ${data.walletAddress}`, 'ready');
        } else {
          setStatus(`Init failed: ${data.error ?? resp.status}`, 'error');
        }
      } catch (err) {
        setStatus(`Init request failed: ${String(err)}`, 'error');
      }
    })();
  });

  void refreshStatus();
  window.setInterval(() => void refreshStatus(), 3000);
}

if (typeof document !== 'undefined') {
  init();
}
