#!/bin/bash

# MVM Blockchain API Test Script v2
# Tests all endpoints including account, tx lookup, fees

BASE_URL="http://localhost:8545"

echo "=========================================="
echo "🚀 MVM BLOCKCHAIN API TEST v2"
echo "=========================================="
echo ""

# 1. Index
echo "1️⃣  GET / - API Info"
curl -s $BASE_URL/ | jq .
echo ""

# 2. Status
echo "2️⃣  GET /status - Chain Status"
STATUS=$(curl -s $BASE_URL/status)
echo $STATUS | jq .
MASTER=$(echo $STATUS | jq -r '.chain_id')
echo ""

# 3. Latest Block
echo "3️⃣  GET /block/latest - Latest Block"
curl -s $BASE_URL/block/latest | jq .
echo ""

# 4. Genesis Block
echo "4️⃣  GET /block/0 - Genesis Block"
curl -s $BASE_URL/block/0 | jq .
echo ""

# 5. Create Wallet 1
echo "5️⃣  GET /wallet/new - Create Wallet 1"
WALLET1=$(curl -s $BASE_URL/wallet/new)
echo $WALLET1 | jq .
ADDR1=$(echo $WALLET1 | jq -r '.address')
PRIV1=$(echo $WALLET1 | jq -r '.private_key')
PUB1=$(echo $WALLET1 | jq -r '.public_key')
echo "   📝 WALLET1: $ADDR1"
echo ""

# 6. Create Wallet 2
echo "6️⃣  GET /wallet/new - Create Wallet 2"
WALLET2=$(curl -s $BASE_URL/wallet/new)
echo $WALLET2 | jq .
ADDR2=$(echo $WALLET2 | jq -r '.address')
PRIV2=$(echo $WALLET2 | jq -r '.private_key')
PUB2=$(echo $WALLET2 | jq -r '.public_key')
echo "   📝 WALLET2: $ADDR2"
echo ""

# 7. Faucet to Wallet 1
echo "7️⃣  POST /faucet - Faucet to Wallet 1"
curl -s -X POST $BASE_URL/faucet/$ADDR1 | jq .
echo ""

# 8. Balance Check
echo "8️⃣  GET /balance - Wallet 1 Balance"
curl -s $BASE_URL/balance/$ADDR1 | jq .
echo ""

# 9. Nonce Check
echo "9️⃣  GET /nonce - Wallet 1 Nonce"
curl -s $BASE_URL/nonce/$ADDR1 | jq .
echo ""

# 10. Sign Transfer TX
echo "🔟 POST /tx/sign - Sign Transfer TX (Wallet1 -> Wallet2)"
SIGN1=$(curl -s -X POST $BASE_URL/tx/sign \
  -H "Content-Type: application/json" \
  -d "{
    \"private_key\": \"$PRIV1\",
    \"tx_type\": \"transfer\",
    \"from\": \"$ADDR1\",
    \"to\": \"$ADDR2\",
    \"value\": 10,
    \"nonce\": 0
  }")
echo $SIGN1 | jq .
SIG1=$(echo $SIGN1 | jq -r '.signature')
echo ""

# 11. Submit Transfer TX
echo "1️⃣1️⃣ POST /tx - Submit Transfer"
TX1=$(curl -s -X POST $BASE_URL/tx \
  -H "Content-Type: application/json" \
  -d "{
    \"tx_type\": \"transfer\",
    \"from\": \"$ADDR1\",
    \"to\": \"$ADDR2\",
    \"value\": 10,
    \"nonce\": 0,
    \"signature\": \"$SIG1\",
    \"public_key\": \"$PUB1\"
  }")
echo $TX1 | jq .
TX1_HASH=$(echo $TX1 | jq -r '.hash')
echo "   📝 TX HASH: $TX1_HASH"
echo ""

# 12. Wait for block
echo "⏳ Waiting 4 seconds for block..."
sleep 4
echo ""

# 13. Get Transaction by Hash
echo "1️⃣2️⃣ GET /tx/:hash - Transaction Details"
curl -s $BASE_URL/tx/$TX1_HASH | jq .
echo ""

# 14. Check Balances After Transfer
echo "1️⃣3️⃣ Balances After Transfer"
echo "   Wallet 1:"
curl -s $BASE_URL/balance/$ADDR1 | jq .
echo "   Wallet 2:"
curl -s $BASE_URL/balance/$ADDR2 | jq .
echo ""

# 15. Sign Create Token TX
echo "1️⃣4️⃣ POST /tx/sign - Sign Create Token TX"
NONCE1=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN2=$(curl -s -X POST $BASE_URL/tx/sign \
  -H "Content-Type: application/json" \
  -d "{
    \"private_key\": \"$PRIV1\",
    \"tx_type\": \"create_token\",
    \"from\": \"$ADDR1\",
    \"value\": 0,
    \"nonce\": $NONCE1,
    \"data\": {
      \"name\": \"Pepe Token\",
      \"symbol\": \"PEPE\",
      \"total_supply\": 1000000
    }
  }")
echo $SIGN2 | jq .
SIG2=$(echo $SIGN2 | jq -r '.signature')
echo ""

# 16. Submit Create Token TX
echo "1️⃣5️⃣ POST /tx - Create Token"
TX2=$(curl -s -X POST $BASE_URL/tx \
  -H "Content-Type: application/json" \
  -d "{
    \"tx_type\": \"create_token\",
    \"from\": \"$ADDR1\",
    \"value\": 0,
    \"nonce\": $NONCE1,
    \"data\": {
      \"name\": \"Pepe Token\",
      \"symbol\": \"PEPE\",
      \"total_supply\": 1000000
    },
    \"signature\": \"$SIG2\",
    \"public_key\": \"$PUB1\"
  }")
echo $TX2 | jq .
TX2_HASH=$(echo $TX2 | jq -r '.hash')
echo ""

# 17. Wait for block
echo "⏳ Waiting 4 seconds for block..."
sleep 4
echo ""

# 18. Get Token TX Details
echo "1️⃣6️⃣ GET /tx/:hash - Token Creation TX Details"
curl -s $BASE_URL/tx/$TX2_HASH | jq .
echo ""

# 19. Get All Tokens
echo "1️⃣7️⃣ GET /tokens - All Tokens"
TOKENS=$(curl -s $BASE_URL/tokens)
echo $TOKENS | jq .
TOKEN_ADDR=$(echo $TOKENS | jq -r '.tokens[0].address // empty')
echo ""

# 20. Get Token Info
if [ -n "$TOKEN_ADDR" ]; then
  echo "1️⃣8️⃣ GET /token/:address - Token Info"
  curl -s $BASE_URL/token/$TOKEN_ADDR | jq .
  echo ""

  # 21. Get Token Balance
  echo "1️⃣9️⃣ GET /token/:contract/balance/:address - Token Balance"
  curl -s $BASE_URL/token/$TOKEN_ADDR/balance/$ADDR1 | jq .
  echo ""
fi

# 22. Get Tokens by Creator
echo "2️⃣0️⃣ GET /tokens/creator/:address - Tokens Created by Wallet 1"
curl -s $BASE_URL/tokens/creator/$ADDR1 | jq .
echo ""

# 23. Get Token Holdings
echo "2️⃣1️⃣ GET /tokens/holder/:address - Token Holdings of Wallet 1"
curl -s $BASE_URL/tokens/holder/$ADDR1 | jq .
echo ""

# 24. Get All Transactions for Address
echo "2️⃣2️⃣ GET /txs/:address - All Transactions for Wallet 1"
curl -s $BASE_URL/txs/$ADDR1 | jq .
echo ""

# 25. Get Full Account Info
echo "2️⃣3️⃣ GET /account/:address - Full Account Details (Wallet 1)"
curl -s $BASE_URL/account/$ADDR1 | jq .
echo ""

# 26. Transfer Token (if token exists)
if [ -n "$TOKEN_ADDR" ]; then
  echo "2️⃣4️⃣ POST /tx/sign - Sign Token Transfer TX"
  NONCE1=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
  SIGN3=$(curl -s -X POST $BASE_URL/tx/sign \
    -H "Content-Type: application/json" \
    -d "{
      \"private_key\": \"$PRIV1\",
      \"tx_type\": \"transfer_token\",
      \"from\": \"$ADDR1\",
      \"value\": 0,
      \"nonce\": $NONCE1,
      \"data\": {
        \"contract\": \"$TOKEN_ADDR\",
        \"to\": \"$ADDR2\",
        \"amount\": 5000
      }
    }")
  echo $SIGN3 | jq .
  SIG3=$(echo $SIGN3 | jq -r '.signature')
  echo ""

  echo "2️⃣5️⃣ POST /tx - Transfer Token"
  TX3=$(curl -s -X POST $BASE_URL/tx \
    -H "Content-Type: application/json" \
    -d "{
      \"tx_type\": \"transfer_token\",
      \"from\": \"$ADDR1\",
      \"value\": 0,
      \"nonce\": $NONCE1,
      \"data\": {
        \"contract\": \"$TOKEN_ADDR\",
        \"to\": \"$ADDR2\",
        \"amount\": 5000
      },
      \"signature\": \"$SIG3\",
      \"public_key\": \"$PUB1\"
    }")
  echo $TX3 | jq .
  echo ""

  echo "⏳ Waiting 4 seconds for block..."
  sleep 4
  echo ""

  # Check token balances
  echo "2️⃣6️⃣ Token Balances After Transfer"
  echo "   Wallet 1 PEPE:"
  curl -s $BASE_URL/token/$TOKEN_ADDR/balance/$ADDR1 | jq .
  echo "   Wallet 2 PEPE:"
  curl -s $BASE_URL/token/$TOKEN_ADDR/balance/$ADDR2 | jq .
  echo ""
fi

# 27. Final Account Summary
echo "2️⃣7️⃣ Final Account Summary - Wallet 1"
curl -s $BASE_URL/account/$ADDR1 | jq .
echo ""

echo "2️⃣8️⃣ Final Account Summary - Wallet 2"
curl -s $BASE_URL/account/$ADDR2 | jq .
echo ""

# 28. Latest Block with All TXs
echo "2️⃣9️⃣ GET /block/latest - Latest Block"
curl -s $BASE_URL/block/latest | jq .
echo ""

echo "=========================================="
echo "✅ ALL TESTS COMPLETE"
echo "=========================================="
echo ""
echo "📊 SUMMARY:"
echo "   Wallet 1: $ADDR1"
echo "   Wallet 2: $ADDR2"
if [ -n "$TOKEN_ADDR" ]; then
  echo "   Token:    $TOKEN_ADDR (PEPE)"
fi
echo ""
echo "🔗 Quick Links:"
echo "   Account 1: $BASE_URL/account/$ADDR1"
echo "   Account 2: $BASE_URL/account/$ADDR2"
echo "   All Tokens: $BASE_URL/tokens"
echo ""