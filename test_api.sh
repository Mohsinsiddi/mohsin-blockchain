#!/bin/bash

# MVM Blockchain API Test Script
# Master Address: mvm11ymqxyuad5udhjykzwqemztc3zzg0ttvsf66swl

BASE_URL="http://localhost:8545"
MASTER="mvm11gcze8v9m3l9lw9xwvjhu2cu8xphkjxp5vqmlr3"

echo "=========================================="
echo "🚀 MVM BLOCKCHAIN API TEST"
echo "=========================================="
echo ""

# 1. Index
echo "1️⃣  GET / - API Info"
curl -s $BASE_URL/ | jq .
echo ""

# 2. Status
echo "2️⃣  GET /status - Chain Status"
curl -s $BASE_URL/status | jq .
echo ""

# 3. Latest Block
echo "3️⃣  GET /block/latest - Latest Block"
curl -s $BASE_URL/block/latest | jq .
echo ""

# 4. Block 0 (Genesis)
echo "4️⃣  GET /block/0 - Genesis Block"
curl -s $BASE_URL/block/0 | jq .
echo ""

# 5. Master Balance
echo "5️⃣  GET /balance - Master Balance"
curl -s $BASE_URL/balance/$MASTER | jq .
echo ""

# 6. Master Nonce
echo "6️⃣  GET /nonce - Master Nonce"
curl -s $BASE_URL/nonce/$MASTER | jq .
echo ""

# 7. Create New Wallet
echo "7️⃣  GET /wallet/new - Create Wallet"
WALLET_RESPONSE=$(curl -s $BASE_URL/wallet/new)
echo $WALLET_RESPONSE | jq .
NEW_ADDRESS=$(echo $WALLET_RESPONSE | jq -r '.address')
NEW_PRIVATE_KEY=$(echo $WALLET_RESPONSE | jq -r '.private_key')
NEW_PUBLIC_KEY=$(echo $WALLET_RESPONSE | jq -r '.public_key')
echo ""
echo "   📝 Saved: NEW_ADDRESS=$NEW_ADDRESS"
echo ""

# 8. Faucet to New Wallet
echo "8️⃣  POST /faucet - Faucet to New Wallet"
curl -s -X POST $BASE_URL/faucet/$NEW_ADDRESS | jq .
echo ""

# 9. New Wallet Balance
echo "9️⃣  GET /balance - New Wallet Balance"
curl -s $BASE_URL/balance/$NEW_ADDRESS | jq .
echo ""

# 10. Sign Transaction (Transfer from new wallet to master)
echo "🔟 POST /tx/sign - Sign Transfer TX"
SIGN_RESPONSE=$(curl -s -X POST $BASE_URL/tx/sign \
  -H "Content-Type: application/json" \
  -d "{
    \"private_key\": \"$NEW_PRIVATE_KEY\",
    \"tx_type\": \"transfer\",
    \"from\": \"$NEW_ADDRESS\",
    \"to\": \"$MASTER\",
    \"value\": 10,
    \"nonce\": 0
  }")
echo $SIGN_RESPONSE | jq .
SIGNATURE=$(echo $SIGN_RESPONSE | jq -r '.signature')
PUBLIC_KEY=$(echo $SIGN_RESPONSE | jq -r '.public_key')
echo ""

# 11. Submit Signed Transfer
echo "1️⃣1️⃣ POST /tx - Submit Transfer"
curl -s -X POST $BASE_URL/tx \
  -H "Content-Type: application/json" \
  -d "{
    \"tx_type\": \"transfer\",
    \"from\": \"$NEW_ADDRESS\",
    \"to\": \"$MASTER\",
    \"value\": 10,
    \"nonce\": 0,
    \"signature\": \"$SIGNATURE\",
    \"public_key\": \"$PUBLIC_KEY\"
  }" | jq .
echo ""

# 12. Wait for block
echo "⏳ Waiting 4 seconds for block..."
sleep 4
echo ""

# 13. Check balances after transfer
echo "1️⃣2️⃣ Balances After Transfer"
echo "   Master:"
curl -s $BASE_URL/balance/$MASTER | jq .
echo "   New Wallet:"
curl -s $BASE_URL/balance/$NEW_ADDRESS | jq .
echo ""

# 14. Sign Create Token TX
echo "1️⃣3️⃣ POST /tx/sign - Sign Create Token TX"
NONCE=$(curl -s $BASE_URL/nonce/$NEW_ADDRESS | jq -r '.nonce')
TOKEN_SIGN=$(curl -s -X POST $BASE_URL/tx/sign \
  -H "Content-Type: application/json" \
  -d "{
    \"private_key\": \"$NEW_PRIVATE_KEY\",
    \"tx_type\": \"create_token\",
    \"from\": \"$NEW_ADDRESS\",
    \"value\": 0,
    \"nonce\": $NONCE,
    \"data\": {
      \"name\": \"Pepe Token\",
      \"symbol\": \"PEPE\",
      \"total_supply\": 1000000
    }
  }")
echo $TOKEN_SIGN | jq .
TOKEN_SIG=$(echo $TOKEN_SIGN | jq -r '.signature')
TOKEN_PK=$(echo $TOKEN_SIGN | jq -r '.public_key')
echo ""

# 15. Submit Create Token TX
echo "1️⃣4️⃣ POST /tx - Create Token"
curl -s -X POST $BASE_URL/tx \
  -H "Content-Type: application/json" \
  -d "{
    \"tx_type\": \"create_token\",
    \"from\": \"$NEW_ADDRESS\",
    \"value\": 0,
    \"nonce\": $NONCE,
    \"data\": {
      \"name\": \"Pepe Token\",
      \"symbol\": \"PEPE\",
      \"total_supply\": 1000000
    },
    \"signature\": \"$TOKEN_SIG\",
    \"public_key\": \"$TOKEN_PK\"
  }" | jq .
echo ""

# 16. Wait for block
echo "⏳ Waiting 4 seconds for block..."
sleep 4
echo ""

# 17. Get All Tokens
echo "1️⃣5️⃣ GET /tokens - All Tokens"
TOKENS=$(curl -s $BASE_URL/tokens)
echo $TOKENS | jq .
TOKEN_ADDRESS=$(echo $TOKENS | jq -r '.tokens[0].address // empty')
echo ""

# 18. Get Token Info
if [ -n "$TOKEN_ADDRESS" ]; then
  echo "1️⃣6️⃣ GET /token/:address - Token Info"
  curl -s $BASE_URL/token/$TOKEN_ADDRESS | jq .
  echo ""

  # 19. Get Token Balance
  echo "1️⃣7️⃣ GET /token/:contract/balance/:address - Token Balance"
  curl -s $BASE_URL/token/$TOKEN_ADDRESS/balance/$NEW_ADDRESS | jq .
  echo ""
fi

# 20. Latest Block (should have TXs)
echo "1️⃣8️⃣ GET /block/latest - Latest Block (with TXs)"
curl -s $BASE_URL/block/latest | jq .
echo ""

echo "=========================================="
echo "✅ ALL TESTS COMPLETE"
echo "=========================================="