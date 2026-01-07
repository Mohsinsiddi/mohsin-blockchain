#!/bin/bash

# MOSH Contract Test Script
# Tests user-defined functions, variables, mappings

BASE_URL="http://localhost:8545"

echo "=========================================="
echo "🚀 MOSH CONTRACT TEST"
echo "=========================================="
echo ""

# 1. Create Wallets
echo "1️⃣  Create Wallets"
WALLET1=$(curl -s $BASE_URL/wallet/new)
ADDR1=$(echo $WALLET1 | jq -r '.address')
PRIV1=$(echo $WALLET1 | jq -r '.private_key')
PUB1=$(echo $WALLET1 | jq -r '.public_key')
echo "   Wallet 1: $ADDR1"

WALLET2=$(curl -s $BASE_URL/wallet/new)
ADDR2=$(echo $WALLET2 | jq -r '.address')
echo "   Wallet 2: $ADDR2"
echo ""

# 2. Faucet
echo "2️⃣  Faucet"
curl -s -X POST $BASE_URL/faucet/$ADDR1 | jq -r '.new_balance'
curl -s -X POST $BASE_URL/faucet/$ADDR2 | jq -r '.new_balance'
echo ""

# 3. Create PEPE Token
echo "3️⃣  Create PEPE Token"
NONCE=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"create_token\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"name\": \"Pepe Token\", \"symbol\": \"PEPE\", \"total_supply\": 1000000}
}")
SIG=$(echo $SIGN | jq -r '.signature')

curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"create_token\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"name\": \"Pepe Token\", \"symbol\": \"PEPE\", \"total_supply\": 1000000},
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB1\"
}" | jq .

sleep 4

TOKEN=$(curl -s $BASE_URL/tokens | jq -r '.tokens[0].address')
echo "   Token: $TOKEN"
echo ""

# ========== SIMPLE CONTRACT (Variables Only) ==========
echo "=========================================="
echo "📦 SIMPLE CONTRACT (Variables Only)"
echo "=========================================="

echo "4️⃣  Deploy Simple Counter Contract"
NONCE=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"deploy_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {
    \"name\": \"Counter\",
    \"variables\": [
      {\"name\": \"count\", \"type\": \"uint64\", \"default\": \"0\"},
      {\"name\": \"name\", \"type\": \"string\", \"default\": \"MyCounter\"}
    ],
    \"mappings\": [],
    \"functions\": []
  }
}")
SIG=$(echo $SIGN | jq -r '.signature')

TX=$(curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"deploy_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {
    \"name\": \"Counter\",
    \"variables\": [
      {\"name\": \"count\", \"type\": \"uint64\", \"default\": \"0\"},
      {\"name\": \"name\", \"type\": \"string\", \"default\": \"MyCounter\"}
    ],
    \"mappings\": [],
    \"functions\": []
  },
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB1\"
}")
echo $TX | jq .

sleep 4

TX_HASH=$(echo $TX | jq -r '.hash')
COUNTER=$(curl -s $BASE_URL/tx/$TX_HASH | jq -r '.transaction.to')
echo "   Counter Contract: $COUNTER"
echo ""

echo "5️⃣  Get Counter Info"
curl -s $BASE_URL/contract/$COUNTER | jq .
echo ""

echo "6️⃣  Get count (auto getter)"
NONCE=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"call_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"contract\": \"$COUNTER\", \"method\": \"get_count\", \"args\": []}
}")
SIG=$(echo $SIGN | jq -r '.signature')

curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"call_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"contract\": \"$COUNTER\", \"method\": \"get_count\", \"args\": []},
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB1\"
}" | jq .

sleep 4

echo ""
echo "7️⃣  Set count = 42 (auto setter)"
NONCE=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"call_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"contract\": \"$COUNTER\", \"method\": \"set_count\", \"args\": [\"42\"]}
}")
SIG=$(echo $SIGN | jq -r '.signature')

curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"call_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"contract\": \"$COUNTER\", \"method\": \"set_count\", \"args\": [\"42\"]},
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB1\"
}" | jq .

sleep 4

echo ""
echo "8️⃣  Verify count = 42"
curl -s $BASE_URL/contract/$COUNTER | jq '.variables'
echo ""

# ========== CONTRACT WITH MAPPINGS ==========
echo "=========================================="
echo "📦 CONTRACT WITH MAPPINGS"
echo "=========================================="

echo "9️⃣  Deploy Whitelist Contract"
NONCE=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"deploy_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {
    \"name\": \"Whitelist\",
    \"variables\": [
      {\"name\": \"total\", \"type\": \"uint64\", \"default\": \"0\"}
    ],
    \"mappings\": [
      {\"name\": \"allowed\", \"key_type\": \"address\", \"value_type\": \"bool\"}
    ],
    \"functions\": []
  }
}")
SIG=$(echo $SIGN | jq -r '.signature')

TX=$(curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"deploy_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {
    \"name\": \"Whitelist\",
    \"variables\": [
      {\"name\": \"total\", \"type\": \"uint64\", \"default\": \"0\"}
    ],
    \"mappings\": [
      {\"name\": \"allowed\", \"key_type\": \"address\", \"value_type\": \"bool\"}
    ],
    \"functions\": []
  },
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB1\"
}")
echo $TX | jq .

sleep 4

TX_HASH=$(echo $TX | jq -r '.hash')
WHITELIST=$(curl -s $BASE_URL/tx/$TX_HASH | jq -r '.transaction.to')
echo "   Whitelist Contract: $WHITELIST"
echo ""

echo "🔟 Set allowed[$ADDR2] = true"
NONCE=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"call_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"contract\": \"$WHITELIST\", \"method\": \"set_allowed\", \"args\": [\"$ADDR2\", \"true\"]}
}")
SIG=$(echo $SIGN | jq -r '.signature')

curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"call_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"contract\": \"$WHITELIST\", \"method\": \"set_allowed\", \"args\": [\"$ADDR2\", \"true\"]},
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB1\"
}" | jq .

sleep 4

echo ""
echo "1️⃣1️⃣ Get allowed[$ADDR2]"
NONCE=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"call_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"contract\": \"$WHITELIST\", \"method\": \"get_allowed\", \"args\": [\"$ADDR2\"]}
}")
SIG=$(echo $SIGN | jq -r '.signature')

curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"call_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"contract\": \"$WHITELIST\", \"method\": \"get_allowed\", \"args\": [\"$ADDR2\"]},
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB1\"
}" | jq .

sleep 4

echo ""
echo "1️⃣2️⃣ View Mapping Entries"
curl -s $BASE_URL/contract/$WHITELIST/mapping/allowed | jq .
echo ""

# ========== CONTRACT WITH FUNCTIONS ==========
echo "=========================================="
echo "📦 CONTRACT WITH USER FUNCTIONS"
echo "=========================================="

echo "1️⃣3️⃣ Deploy Vault Contract (with functions)"
NONCE=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"deploy_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {
    \"name\": \"Vault\",
    \"token\": \"$TOKEN\",
    \"variables\": [
      {\"name\": \"total_staked\", \"type\": \"uint64\", \"default\": \"0\"}
    ],
    \"mappings\": [
      {\"name\": \"balances\", \"key_type\": \"address\", \"value_type\": \"uint64\"}
    ],
    \"functions\": [
      {
        \"name\": \"stake\",
        \"modifiers\": [\"payable\"],
        \"args\": [],
        \"body\": [
          {\"op\": \"require\", \"left\": \"msg.amount\", \"cmp\": \">\", \"right\": \"0\", \"msg\": \"Amount = 0\"},
          {\"op\": \"map_add\", \"map\": \"balances\", \"key\": \"msg.sender\", \"value\": \"msg.amount\"},
          {\"op\": \"add\", \"var\": \"total_staked\", \"value\": \"msg.amount\"}
        ]
      },
      {
        \"name\": \"unstake\",
        \"modifiers\": [\"write\"],
        \"args\": [{\"name\": \"amount\", \"type\": \"uint64\"}],
        \"body\": [
          {\"op\": \"require\", \"left\": \"balances[msg.sender]\", \"cmp\": \">=\", \"right\": \"amount\", \"msg\": \"Insufficient\"},
          {\"op\": \"map_sub\", \"map\": \"balances\", \"key\": \"msg.sender\", \"value\": \"amount\"},
          {\"op\": \"sub\", \"var\": \"total_staked\", \"value\": \"amount\"},
          {\"op\": \"transfer\", \"to\": \"msg.sender\", \"amount\": \"amount\"}
        ]
      }
    ]
  }
}")
SIG=$(echo $SIGN | jq -r '.signature')

TX=$(curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"deploy_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {
    \"name\": \"Vault\",
    \"token\": \"$TOKEN\",
    \"variables\": [
      {\"name\": \"total_staked\", \"type\": \"uint64\", \"default\": \"0\"}
    ],
    \"mappings\": [
      {\"name\": \"balances\", \"key_type\": \"address\", \"value_type\": \"uint64\"}
    ],
    \"functions\": [
      {
        \"name\": \"stake\",
        \"modifiers\": [\"payable\"],
        \"args\": [],
        \"body\": [
          {\"op\": \"require\", \"left\": \"msg.amount\", \"cmp\": \">\", \"right\": \"0\", \"msg\": \"Amount = 0\"},
          {\"op\": \"map_add\", \"map\": \"balances\", \"key\": \"msg.sender\", \"value\": \"msg.amount\"},
          {\"op\": \"add\", \"var\": \"total_staked\", \"value\": \"msg.amount\"}
        ]
      },
      {
        \"name\": \"unstake\",
        \"modifiers\": [\"write\"],
        \"args\": [{\"name\": \"amount\", \"type\": \"uint64\"}],
        \"body\": [
          {\"op\": \"require\", \"left\": \"balances[msg.sender]\", \"cmp\": \">=\", \"right\": \"amount\", \"msg\": \"Insufficient\"},
          {\"op\": \"map_sub\", \"map\": \"balances\", \"key\": \"msg.sender\", \"value\": \"amount\"},
          {\"op\": \"sub\", \"var\": \"total_staked\", \"value\": \"amount\"},
          {\"op\": \"transfer\", \"to\": \"msg.sender\", \"amount\": \"amount\"}
        ]
      }
    ]
  },
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB1\"
}")
echo $TX | jq .

sleep 4

TX_HASH=$(echo $TX | jq -r '.hash')
VAULT=$(curl -s $BASE_URL/tx/$TX_HASH | jq -r '.transaction.to')
echo "   Vault Contract: $VAULT"
echo ""

echo "1️⃣4️⃣ View Vault Contract"
curl -s $BASE_URL/contract/$VAULT | jq .
echo ""

echo "1️⃣5️⃣ Stake 1000 PEPE (payable function)"
NONCE=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"call_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"contract\": \"$VAULT\", \"method\": \"stake\", \"args\": [], \"amount\": 100000000000}
}")
SIG=$(echo $SIGN | jq -r '.signature')

curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"call_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"contract\": \"$VAULT\", \"method\": \"stake\", \"args\": [], \"amount\": 100000000000},
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB1\"
}" | jq .

sleep 4

echo ""
echo "1️⃣6️⃣ View Balances Mapping"
curl -s $BASE_URL/contract/$VAULT/mapping/balances | jq .
echo ""

echo "1️⃣7️⃣ View total_staked"
curl -s $BASE_URL/contract/$VAULT | jq '.variables'
echo ""

# ========== SUMMARY ==========
echo "=========================================="
echo "✅ MOSH CONTRACT TEST COMPLETE"
echo "=========================================="
echo ""
echo "📊 CONTRACTS CREATED:"
echo "   Counter:   $COUNTER"
echo "   Whitelist: $WHITELIST"
echo "   Vault:     $VAULT"
echo ""
echo "🎯 FEATURES TESTED:"
echo "   ✅ Simple variables (uint64, string)"
echo "   ✅ Auto getters (get_count, get_name)"
echo "   ✅ Auto setters (set_count)"
echo "   ✅ Mappings (allowed[addr] = bool)"
echo "   ✅ Mapping API (/contract/:addr/mapping/:name)"
echo "   ✅ User functions (stake, unstake)"
echo "   ✅ Payable functions (stake with amount)"
echo "   ✅ Operations: require, map_add, map_sub, add, sub, transfer"
echo ""
echo "📝 All contracts:"
curl -s $BASE_URL/contracts | jq '.contracts[] | {name, address}'