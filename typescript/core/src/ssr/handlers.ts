/**
 * Hyperstack Auth Server - Drop-in Endpoint Handlers
 *
 * These are framework-agnostic API route handlers that users can mount however they like.
 * They handle token minting and JWKS serving directly.
 *
 * @example
 * ```typescript
 * // app/api/hyperstack/sessions/route.ts (Next.js App Router)
 * import { handleSessionRequest, handleJwksRequest } from 'hyperstack-typescript/ssr/handlers';
 *
 * export async function POST() {
 *   return handleSessionRequest();
 * }
 *
 * export async function GET() {
 *   return handleJwksRequest();
 * }
 * ```
 */

import jwt from 'jsonwebtoken';

export interface AuthHandlerConfig {
  /**
   * JWT signing secret (base64-encoded).
   * Set HYPERSTACK_SIGNING_KEY env var OR pass here.
   */
  signingKey?: string;

  /**
   * Token issuer (defaults to HYPERSTACK_ISSUER env var or 'hyperstack')
   */
  issuer?: string;

  /**
   * Token audience (defaults to HYPERSTACK_AUDIENCE env var)
   */
  audience?: string;

  /**
   * Token TTL in seconds (defaults to 300 = 5 minutes)
   */
  ttlSeconds?: number;

  /**
   * Custom limits for tokens
   */
  limits?: {
    max_connections?: number;
    max_subscriptions?: number;
    max_snapshot_rows?: number;
  };
}

export interface SessionClaims {
  iss: string;
  sub: string;
  aud: string;
  iat: number;
  nbf: number;
  exp: number;
  jti: string;
  scope: string;
  metering_key: string;
  key_class: 'secret' | 'publishable';
  limits?: {
    max_connections?: number;
    max_subscriptions?: number;
    max_snapshot_rows?: number;
    max_messages_per_minute?: number;
    max_bytes_per_minute?: number;
  };
}

export interface TokenResponse {
  token: string;
  expires_at: number;
}

export interface JwksResponse {
  keys: Array<{
    kty: string;
    kid: string;
    use: string;
    alg: string;
    x: string;
  }>;
}

/**
 * Mint a session token
 */
export function mintSessionToken(
  config: AuthHandlerConfig,
  subject: string = 'anonymous',
  scope: string = 'read'
): TokenResponse {
  const signingKey = config.signingKey || process.env.HYPERSTACK_SIGNING_KEY;
  if (!signingKey) {
    throw new Error(
      'HYPERSTACK_SIGNING_KEY not set. Generate with: node -e "console.log(require(\'crypto\').randomBytes(32).toString(\'base64\'))"'
    );
  }

  const secret = Buffer.from(signingKey, 'base64');
  const issuer = config.issuer || process.env.HYPERSTACK_ISSUER || 'hyperstack';
  const audience = config.audience || process.env.HYPERSTACK_AUDIENCE || 'hyperstack';
  const ttlSeconds = config.ttlSeconds || 300;
  
  const now = Math.floor(Date.now() / 1000);
  const expiresAt = now + ttlSeconds;
  
  const claims: SessionClaims = {
    iss: issuer,
    sub: subject,
    aud: audience,
    iat: now,
    nbf: now,
    exp: expiresAt,
    jti: `${subject}-${now}`,
    scope,
    metering_key: `meter:${subject}`,
    key_class: 'secret',
    limits: config.limits || {
      max_connections: 10,
      max_subscriptions: 100,
      max_snapshot_rows: 1000,
      max_messages_per_minute: 10000,
      max_bytes_per_minute: 100 * 1024 * 1024,
    },
  };

  const token = jwt.sign(claims, secret, { algorithm: 'HS256' });

  return {
    token,
    expires_at: expiresAt,
  };
}

/**
 * Generate JWKS response from signing key
 */
export function generateJwks(config: AuthHandlerConfig): JwksResponse {
  const signingKey = config.signingKey || process.env.HYPERSTACK_SIGNING_KEY;
  if (!signingKey) {
    return { keys: [] };
  }

  // For HMAC-SHA256, we return the public key info
  // Note: In production, you might want to use asymmetric keys (RS256/ES256)
  // for JWKS, but HS256 is fine for self-hosted setups
  const secret = Buffer.from(signingKey, 'base64');
  const publicKey = secret.toString('base64url');

  return {
    keys: [
      {
        kty: 'oct',
        kid: 'key-1',
        use: 'sig',
        alg: 'HS256',
        x: publicKey,
      },
    ],
  };
}

/**
 * Framework-agnostic request handler for token minting
 * Returns a Response object that can be used with any framework
 */
export function handleSessionRequest(
  config: AuthHandlerConfig = {},
  subject: string = 'anonymous',
  scope: string = 'read'
): Response {
  try {
    const tokenData = mintSessionToken(config, subject, scope);
    
    return new Response(JSON.stringify(tokenData), {
      status: 200,
      headers: {
        'Content-Type': 'application/json',
      },
    });
  } catch (error) {
    return new Response(
      JSON.stringify({
        error: error instanceof Error ? error.message : 'Failed to mint token',
      }),
      {
        status: 500,
        headers: {
          'Content-Type': 'application/json',
        },
      }
    );
  }
}

/**
 * Framework-agnostic request handler for JWKS endpoint
 */
export function handleJwksRequest(config: AuthHandlerConfig = {}): Response {
  const jwks = generateJwks(config);
  
  return new Response(JSON.stringify(jwks), {
    status: 200,
    headers: {
      'Content-Type': 'application/json',
    },
  });
}

/**
 * Framework-agnostic health check handler
 */
export function handleHealthRequest(): Response {
  return new Response(
    JSON.stringify({
      status: 'healthy',
      version: '0.5.10',
    }),
    {
      status: 200,
      headers: {
        'Content-Type': 'application/json',
      },
    }
  );
}
