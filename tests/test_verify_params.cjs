// 测试: 模拟 verify_user_info 请求，对比参数
// 用 KuGouMusicApi 源码，输入和 Rust 端相同的输入参数
const crypto = require('../kugou-api-ref/util/crypto');
const helper = require('../kugou-api-ref/util/helper');

// 模拟输入（从 Rust 端拿到的 edt/sid/pk/params）
// 先 mock 一个场景生成 pk/params
const aesResult = crypto.cryptoAesEncrypt({});
console.log('=== cryptoAesEncrypt({}) ===');
console.log('key  (tempKey):', aesResult.key);
console.log('str  (params):', aesResult.str);

const rsaInput = { key: aesResult.key };
const pk = crypto.cryptoRSAEncrypt(rsaInput);
console.log('\n=== cryptoRSAEncrypt({key: "' + aesResult.key + '"}) ===');
console.log('pk:', pk);

// 现在看看我们 Rust 的 generate_pk_params 输出是否类似
// Rust 用 random_hex(16) 作为 temp_key，Js 用 randomString(16).toLowerCase()
// 测试用 Js 的 randomString 生成
const { randomString } = require('../kugou-api-ref/util/util');
const jsTempKey = randomString(16).toLowerCase();
console.log('\n=== Js randomString(16) tempKey ===');
console.log('tempKey:', jsTempKey);
const jsMd5 = crypto.cryptoMd5(jsTempKey);
console.log('md5:', jsMd5);
console.log('key:', jsMd5.substring(0, 32));
console.log('iv:', jsMd5.substring(jsMd5.length - 16));

// 现在测试 RSA raw encrypt 和我们的 Rust 是否一致
const publicLiteRasKey = `-----BEGIN PUBLIC KEY-----\nMIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDECi0Np2UR87scwrvTr72L6oO01rBbbBPriSDFPxr3Z5syug0O24QyQO8bg27+0+4kBzTBTBOZ/WWU0WryL1JSXRTXLgFVxtzIY41Pe7lPOgsfTCn5kZcvKhYKJesKnnJDNr5/abvTGf+rHG3YRwsCHcQ08/q6ifSioBszvb3QiwIDAQAB\n-----END PUBLIC KEY-----`;

const forge = require('node-forge');
const key = forge.pki.publicKeyFromPem(publicLiteRasKey);
const keyLength = Math.ceil(key.n.bitLength() / 8);

// 用已知 key 测试 raw RSA — 用 "test" 字符串
const testData = JSON.stringify({ key: 'test1234567890' });
console.log('\n=== RSA Raw Encrypt 测试 ===');
console.log('input:', testData);
const encoded = Buffer.from(testData, 'utf8');
console.log('utf8 bytes:', Array.from(encoded).map(b => b.toString(16).padStart(2, '0')).join(''));
const padded = Buffer.alloc(keyLength);
encoded.copy(padded);
console.log('padded hex:', padded.toString('hex'));

const message = new forge.jsbn.BigInteger(padded.toString('hex'), 16);
const encrypted = message.modPow(key.e, key.n);
const hexResult = encrypted.toString(16).padStart(keyLength * 2, '0');
console.log('RSA raw result:', hexResult);
