#!/bin/bash

# ============================================================
# MVM COMPREHENSIVE API TEST SCRIPT
# Tests ALL endpoints, signature verification, free reads
# ============================================================

BASE_URL="http://localhost:8545"
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PASS=0
FAIL=0

# Test helper
test_endpoint() {
    local name="$1"
    local result="$2"
    local expected="$3"
    
    if echo "$result" | grep -q "$expected"; then
        echo -e "   ${GREEN}✓${NC} $name"
        ((PASS++))
    else
        echo -e "   ${RED}✗${NC} $name"
        echo "      Expected: $expected"
        echo "      Got: $(echo $result | head -c 200)"
        ((FAIL++))
    fi
}

echo ""
echo "============================================================"
echo "🚀 MVM COMPREHENSIVE API TEST"
echo "============================================================"
echo ""

# ============================================================
# SECTION 1: CHAIN INFO
# ============================================================
echo -e "${BLUE}━━━ SECTION 1: CHAIN INFO ━━━${NC}"

echo "Testing GET /"
RESULT=$(curl -s $BASE_URL/)
test_endpoint "Root endpoint" "$RESULT" '"name":"MOHSIN VIRTUAL MACHINE"'

echo "Testing GET /status"
RESULT=$(curl -s $BASE_URL/status)
test_endpoint "Status endpoint" "$RESULT" '"chain_id"'

echo "Testing GET /block/latest"
RESULT=$(curl -s $BASE_URL/block/latest)
test_endpoint "Latest block" "$RESULT" '"height"'

echo "Testing GET /block/1"
RESULT=$(curl -s $BASE_URL/block/1)
test_endpoint "Block by height" "$RESULT" '"block"'

echo "Testing GET /blocks?limit=5"
RESULT=$(curl -s "$BASE_URL/blocks?limit=5")
test_endpoint "Recent blocks" "$RESULT" '"blocks"'

echo "Testing GET /txs?limit=5"
RESULT=$(curl -s "$BASE_URL/txs?limit=5")
test_endpoint "Recent transactions" "$RESULT" '"transactions"'

echo ""

# ============================================================
# SECTION 2: WALLET & ACCOUNTS
# ============================================================
echo -e "${BLUE}━━━ SECTION 2: WALLET & ACCOUNTS ━━━${NC}"

echo "Testing GET /wallet/new (Wallet 1)"
WALLET1=$(curl -s $BASE_URL/wallet/new)
ADDR1=$(echo $WALLET1 | jq -r '.address')
PRIV1=$(echo $WALLET1 | jq -r '.private_key')
PUB1=$(echo $WALLET1 | jq -r '.public_key')
test_endpoint "Create wallet 1" "$WALLET1" '"address":"mvm1'

echo "Testing GET /wallet/new (Wallet 2)"
WALLET2=$(curl -s $BASE_URL/wallet/new)
ADDR2=$(echo $WALLET2 | jq -r '.address')
PRIV2=$(echo $WALLET2 | jq -r '.private_key')
PUB2=$(echo $WALLET2 | jq -r '.public_key')
test_endpoint "Create wallet 2" "$WALLET2" '"address":"mvm1'

echo "Testing GET /balance/$ADDR1"
RESULT=$(curl -s $BASE_URL/balance/$ADDR1)
test_endpoint "Get balance (should be 0)" "$RESULT" '"balance"'

echo "Testing GET /nonce/$ADDR1"
RESULT=$(curl -s $BASE_URL/nonce/$ADDR1)
test_endpoint "Get nonce" "$RESULT" '"nonce"'

echo "Testing POST /faucet/$ADDR1"
RESULT=$(curl -s -X POST $BASE_URL/faucet/$ADDR1)
test_endpoint "Faucet wallet 1" "$RESULT" '"success":true'

echo "Testing POST /faucet/$ADDR2"
RESULT=$(curl -s -X POST $BASE_URL/faucet/$ADDR2)
test_endpoint "Faucet wallet 2" "$RESULT" '"success":true'

echo "Testing GET /account/$ADDR1"
RESULT=$(curl -s $BASE_URL/account/$ADDR1)
test_endpoint "Get account info" "$RESULT" '"balance"'

echo ""

# ============================================================
# SECTION 3: SIGNATURE VERIFICATION
# ============================================================
echo -e "${BLUE}━━━ SECTION 3: SIGNATURE VERIFICATION ━━━${NC}"

echo "Testing correct signer = payer..."
NONCE=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"transfer\",
  \"from\": \"$ADDR1\",
  \"to\": \"$ADDR2\",
  \"value\": 1,
  \"nonce\": $NONCE
}")
SIG=$(echo $SIGN | jq -r '.signature')
test_endpoint "Sign transaction" "$SIGN" '"signature"'

RESULT=$(curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"transfer\",
  \"from\": \"$ADDR1\",
  \"to\": \"$ADDR2\",
  \"value\": 1,
  \"nonce\": $NONCE,
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB1\"
}")
test_endpoint "Transfer with correct signer" "$RESULT" '"success":true'

sleep 4

echo "Testing WRONG signer (sign with key1, submit as key2)..."
NONCE=$(curl -s $BASE_URL/nonce/$ADDR2 | jq -r '.nonce')
# Sign with wallet 1's key but claim it's from wallet 2
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"transfer\",
  \"from\": \"$ADDR2\",
  \"to\": \"$ADDR1\",
  \"value\": 1,
  \"nonce\": $NONCE
}")
SIG=$(echo $SIGN | jq -r '.signature')

RESULT=$(curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"transfer\",
  \"from\": \"$ADDR2\",
  \"to\": \"$ADDR1\",
  \"value\": 1,
  \"nonce\": $NONCE,
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB1\"
}")
test_endpoint "Transfer with WRONG signer (should fail)" "$RESULT" '"error"'

echo "Testing public_key mismatch..."
NONCE=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"transfer\",
  \"from\": \"$ADDR1\",
  \"to\": \"$ADDR2\",
  \"value\": 1,
  \"nonce\": $NONCE
}")
SIG=$(echo $SIGN | jq -r '.signature')

# Use wrong public key
RESULT=$(curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"transfer\",
  \"from\": \"$ADDR1\",
  \"to\": \"$ADDR2\",
  \"value\": 1,
  \"nonce\": $NONCE,
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB2\"
}")
test_endpoint "Transfer with wrong public_key (should fail)" "$RESULT" '"error"'

echo ""

# ============================================================
# SECTION 4: TOKENS
# ============================================================
echo -e "${BLUE}━━━ SECTION 4: MVM-20 TOKENS ━━━${NC}"

echo "Testing create_token..."
NONCE=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"create_token\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"name\": \"Test Token\", \"symbol\": \"TEST\", \"total_supply\": 1000000}
}")
SIG=$(echo $SIGN | jq -r '.signature')

RESULT=$(curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"create_token\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"name\": \"Test Token\", \"symbol\": \"TEST\", \"total_supply\": 1000000},
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB1\"
}")
test_endpoint "Create token" "$RESULT" '"success":true'
TX_HASH=$(echo $RESULT | jq -r '.hash')

sleep 4

echo "Testing GET /tokens"
RESULT=$(curl -s $BASE_URL/tokens)
test_endpoint "List tokens" "$RESULT" '"tokens"'
TOKEN=$(echo $RESULT | jq -r '.tokens[0].address')

echo "Testing GET /token/$TOKEN"
RESULT=$(curl -s $BASE_URL/token/$TOKEN)
test_endpoint "Token info" "$RESULT" '"symbol":"TEST"'

echo "Testing GET /token/$TOKEN/balance/$ADDR1"
RESULT=$(curl -s $BASE_URL/token/$TOKEN/balance/$ADDR1)
test_endpoint "Token balance" "$RESULT" '"balance"'

echo "Testing GET /tokens/creator/$ADDR1"
RESULT=$(curl -s $BASE_URL/tokens/creator/$ADDR1)
test_endpoint "Tokens by creator" "$RESULT" '"tokens"'

echo "Testing GET /tokens/holder/$ADDR1"
RESULT=$(curl -s $BASE_URL/tokens/holder/$ADDR1)
test_endpoint "Tokens by holder" "$RESULT" '"holdings"'

echo "Testing transfer_token..."
NONCE=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"transfer_token\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"contract\": \"$TOKEN\", \"to\": \"$ADDR2\", \"amount\": 1000}
}")
SIG=$(echo $SIGN | jq -r '.signature')

RESULT=$(curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"transfer_token\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"contract\": \"$TOKEN\", \"to\": \"$ADDR2\", \"amount\": 1000},
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB1\"
}")
test_endpoint "Transfer token" "$RESULT" '"success":true'

sleep 4

echo ""

# ============================================================
# SECTION 5: SIMPLE CONTRACT
# ============================================================
echo -e "${BLUE}━━━ SECTION 5: SIMPLE CONTRACT ━━━${NC}"

echo "Deploying simple Counter contract..."
NONCE=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"deploy_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {
    \"name\": \"Counter\",
    \"variables\": [{\"name\": \"count\", \"type\": \"uint64\", \"default\": \"0\"}]
  }
}")
SIG=$(echo $SIGN | jq -r '.signature')

RESULT=$(curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"deploy_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {
    \"name\": \"Counter\",
    \"variables\": [{\"name\": \"count\", \"type\": \"uint64\", \"default\": \"0\"}]
  },
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB1\"
}")
test_endpoint "Deploy Counter contract" "$RESULT" '"success":true'
TX_HASH=$(echo $RESULT | jq -r '.hash')

sleep 4

COUNTER=$(curl -s $BASE_URL/tx/$TX_HASH | jq -r '.transaction.to')
echo "   Counter address: $COUNTER"

echo "Testing GET /contracts"
RESULT=$(curl -s $BASE_URL/contracts)
test_endpoint "List contracts" "$RESULT" '"contracts"'

echo "Testing GET /contract/$COUNTER"
RESULT=$(curl -s $BASE_URL/contract/$COUNTER)
test_endpoint "Contract info" "$RESULT" '"name":"Counter"'

echo ""

# ============================================================
# SECTION 6: FREE READS (No signature needed!)
# ============================================================
echo -e "${BLUE}━━━ SECTION 6: FREE READS (No signature!) ━━━${NC}"

echo "Testing GET /contract/$COUNTER/var/count (FREE)"
RESULT=$(curl -s $BASE_URL/contract/$COUNTER/var/count)
test_endpoint "Read variable (FREE)" "$RESULT" '"value":0'

echo "Testing GET /contract/$COUNTER/var/owner (FREE)"
RESULT=$(curl -s $BASE_URL/contract/$COUNTER/var/owner)
test_endpoint "Read owner (FREE)" "$RESULT" '"value":"mvm1'

echo "Testing GET /contract/$COUNTER/call/get_count (FREE)"
RESULT=$(curl -s $BASE_URL/contract/$COUNTER/call/get_count)
test_endpoint "Call get_count (FREE)" "$RESULT" '"result":0'

echo "Testing GET /contract/$COUNTER/call/get_owner (FREE)"
RESULT=$(curl -s $BASE_URL/contract/$COUNTER/call/get_owner)
test_endpoint "Call get_owner (FREE)" "$RESULT" '"result":"mvm1'

echo "Testing GET /contract/$COUNTER/mbi (MBI)"
RESULT=$(curl -s $BASE_URL/contract/$COUNTER/mbi)
test_endpoint "Get MBI" "$RESULT" '"mbi"'

echo ""

# ============================================================
# SECTION 7: CONTRACT WITH MAPPINGS
# ============================================================
echo -e "${BLUE}━━━ SECTION 7: CONTRACT WITH MAPPINGS ━━━${NC}"

echo "Deploying Whitelist contract..."
NONCE=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"deploy_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {
    \"name\": \"Whitelist\",
    \"mappings\": [{\"name\": \"allowed\", \"key_type\": \"address\", \"value_type\": \"bool\"}]
  }
}")
SIG=$(echo $SIGN | jq -r '.signature')

RESULT=$(curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"deploy_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {
    \"name\": \"Whitelist\",
    \"mappings\": [{\"name\": \"allowed\", \"key_type\": \"address\", \"value_type\": \"bool\"}]
  },
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB1\"
}")
test_endpoint "Deploy Whitelist contract" "$RESULT" '"success":true'
TX_HASH=$(echo $RESULT | jq -r '.hash')

sleep 4

WHITELIST=$(curl -s $BASE_URL/tx/$TX_HASH | jq -r '.transaction.to')
echo "   Whitelist address: $WHITELIST"

echo "Setting allowed[$ADDR2] = true..."
NONCE=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"call_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"contract\": \"$WHITELIST\", \"method\": \"set_allowed\", \"args\": [\"$ADDR2\", \"true\"]}
}")
SIG=$(echo $SIGN | jq -r '.signature')

RESULT=$(curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"call_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"contract\": \"$WHITELIST\", \"method\": \"set_allowed\", \"args\": [\"$ADDR2\", \"true\"]},
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB1\"
}")
test_endpoint "Set mapping value" "$RESULT" '"success":true'

sleep 4

echo "Testing GET /contract/$WHITELIST/mapping/allowed (list all)"
RESULT=$(curl -s $BASE_URL/contract/$WHITELIST/mapping/allowed)
test_endpoint "List mapping entries" "$RESULT" '"entries"'

echo "Testing GET /contract/$WHITELIST/mapping/allowed/$ADDR2 (FREE)"
RESULT=$(curl -s $BASE_URL/contract/$WHITELIST/mapping/allowed/$ADDR2)
test_endpoint "Read mapping value (FREE)" "$RESULT" '"value":true'

echo "Testing GET /contract/$WHITELIST/call/get_allowed?args=$ADDR2 (FREE)"
RESULT=$(curl -s "$BASE_URL/contract/$WHITELIST/call/get_allowed?args=$ADDR2")
test_endpoint "Call get_allowed (FREE)" "$RESULT" '"result":true'

echo ""

# ============================================================
# SECTION 8: CONTRACT WITH FUNCTIONS
# ============================================================
echo -e "${BLUE}━━━ SECTION 8: CONTRACT WITH FUNCTIONS ━━━${NC}"

echo "Deploying Vault contract with stake/unstake..."
NONCE=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"deploy_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {
    \"name\": \"Vault\",
    \"token\": \"$TOKEN\",
    \"variables\": [{\"name\": \"total_staked\", \"type\": \"uint64\", \"default\": \"0\"}],
    \"mappings\": [{\"name\": \"balances\", \"key_type\": \"address\", \"value_type\": \"uint64\"}],
    \"functions\": [
      {
        \"name\": \"stake\",
        \"modifiers\": [\"payable\"],
        \"args\": [],
        \"body\": [
          {\"op\": \"require\", \"left\": \"msg.amount\", \"cmp\": \">\", \"right\": \"0\", \"msg\": \"Amount=0\"},
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
      },
      {
        \"name\": \"get_balance\",
        \"modifiers\": [\"view\"],
        \"args\": [{\"name\": \"user\", \"type\": \"address\"}],
        \"body\": [
          {\"op\": \"return\", \"value\": \"balances[user]\"}
        ],
        \"returns\": \"uint64\"
      }
    ]
  }
}")
SIG=$(echo $SIGN | jq -r '.signature')

RESULT=$(curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"deploy_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {
    \"name\": \"Vault\",
    \"token\": \"$TOKEN\",
    \"variables\": [{\"name\": \"total_staked\", \"type\": \"uint64\", \"default\": \"0\"}],
    \"mappings\": [{\"name\": \"balances\", \"key_type\": \"address\", \"value_type\": \"uint64\"}],
    \"functions\": [
      {
        \"name\": \"stake\",
        \"modifiers\": [\"payable\"],
        \"args\": [],
        \"body\": [
          {\"op\": \"require\", \"left\": \"msg.amount\", \"cmp\": \">\", \"right\": \"0\", \"msg\": \"Amount=0\"},
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
      },
      {
        \"name\": \"get_balance\",
        \"modifiers\": [\"view\"],
        \"args\": [{\"name\": \"user\", \"type\": \"address\"}],
        \"body\": [
          {\"op\": \"return\", \"value\": \"balances[user]\"}
        ],
        \"returns\": \"uint64\"
      }
    ]
  },
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB1\"
}")
test_endpoint "Deploy Vault contract" "$RESULT" '"success":true'
TX_HASH=$(echo $RESULT | jq -r '.hash')

sleep 4

VAULT=$(curl -s $BASE_URL/tx/$TX_HASH | jq -r '.transaction.to')
echo "   Vault address: $VAULT"

echo "Testing MBI for Vault..."
RESULT=$(curl -s $BASE_URL/contract/$VAULT/mbi)
test_endpoint "Vault MBI" "$RESULT" '"functions"'

echo "Staking 10000 tokens (payable function)..."
NONCE=$(curl -s $BASE_URL/nonce/$ADDR1 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV1\",
  \"tx_type\": \"call_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"contract\": \"$VAULT\", \"method\": \"stake\", \"args\": [], \"amount\": 10000}
}")
SIG=$(echo $SIGN | jq -r '.signature')

RESULT=$(curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"call_contract\",
  \"from\": \"$ADDR1\",
  \"nonce\": $NONCE,
  \"data\": {\"contract\": \"$VAULT\", \"method\": \"stake\", \"args\": [], \"amount\": 10000},
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB1\"
}")
STAKE_TX=$(echo $RESULT | jq -r '.hash')
test_endpoint "Stake TX submitted" "$RESULT" '"success":true'

sleep 4

# Check stake TX status
echo "Checking stake TX status..."
STAKE_STATUS=$(curl -s $BASE_URL/tx/$STAKE_TX)
test_endpoint "Stake TX success" "$STAKE_STATUS" '"status":"Success"'

# Debug: show stake TX details if failed
if ! echo "$STAKE_STATUS" | grep -q '"status":"Success"'; then
    echo "   DEBUG: Stake TX details:"
    echo "$STAKE_STATUS" | jq '.transaction.status, .transaction.error // "no error"'
fi

sleep 4

echo "Testing FREE read of balances mapping..."
RESULT=$(curl -s $BASE_URL/contract/$VAULT/mapping/balances/$ADDR1)
test_endpoint "Read stake balance (FREE)" "$RESULT" '"value":10000'

echo "Testing FREE read of total_staked..."
RESULT=$(curl -s $BASE_URL/contract/$VAULT/var/total_staked)
test_endpoint "Read total_staked (FREE)" "$RESULT" '"value":10000'

echo "Testing view function is FREE..."
# Debug: Check what functions the contract has
echo "   DEBUG: Contract functions from MBI:"
curl -s "$BASE_URL/contract/$VAULT/mbi" | jq '.mbi.functions[] | .name'

RESULT=$(curl -s "$BASE_URL/contract/$VAULT/call/get_balance?args=$ADDR1")
test_endpoint "Call view function (FREE)" "$RESULT" '"gas":0'

# Debug if failed
if ! echo "$RESULT" | grep -q '"gas":0'; then
    echo "   DEBUG: View function response:"
    echo "$RESULT" | jq '.'
fi

echo "Testing write function NOT free..."
RESULT=$(curl -s "$BASE_URL/contract/$VAULT/call/unstake?args=1000")
test_endpoint "Write function should fail on read endpoint" "$RESULT" '"error"'

echo ""

# ============================================================
# SECTION 9: ACCESS CONTROL
# ============================================================
echo -e "${BLUE}━━━ SECTION 9: ACCESS CONTROL ━━━${NC}"

echo "Testing non-owner trying to set_count..."
NONCE=$(curl -s $BASE_URL/nonce/$ADDR2 | jq -r '.nonce')
SIGN=$(curl -s -X POST $BASE_URL/tx/sign -H "Content-Type: application/json" -d "{
  \"private_key\": \"$PRIV2\",
  \"tx_type\": \"call_contract\",
  \"from\": \"$ADDR2\",
  \"nonce\": $NONCE,
  \"data\": {\"contract\": \"$COUNTER\", \"method\": \"set_count\", \"args\": [\"999\"]}
}")
SIG=$(echo $SIGN | jq -r '.signature')

RESULT=$(curl -s -X POST $BASE_URL/tx -H "Content-Type: application/json" -d "{
  \"tx_type\": \"call_contract\",
  \"from\": \"$ADDR2\",
  \"nonce\": $NONCE,
  \"data\": {\"contract\": \"$COUNTER\", \"method\": \"set_count\", \"args\": [\"999\"]},
  \"signature\": \"$SIG\",
  \"public_key\": \"$PUB2\"
}")
NON_OWNER_TX=$(echo $RESULT | jq -r '.hash')
test_endpoint "Non-owner TX submitted" "$RESULT" '"success":true'

sleep 4

# Check that the TX failed during execution
echo "Checking TX status (should be Failed)..."
TX_STATUS=$(curl -s $BASE_URL/tx/$NON_OWNER_TX)
test_endpoint "TX execution failed (owner check)" "$TX_STATUS" '"status":"Failed"'

echo "Verifying count unchanged..."
RESULT=$(curl -s $BASE_URL/contract/$COUNTER/var/count)
test_endpoint "Count still 0" "$RESULT" '"value":0'

echo ""

# ============================================================
# SECTION 10: TRANSACTION HISTORY
# ============================================================
echo -e "${BLUE}━━━ SECTION 10: TRANSACTION HISTORY ━━━${NC}"

echo "Testing GET /tx/$TX_HASH"
RESULT=$(curl -s $BASE_URL/tx/$TX_HASH)
test_endpoint "Get transaction by hash" "$RESULT" '"transaction"'

echo "Testing GET /txs/$ADDR1"
RESULT=$(curl -s $BASE_URL/txs/$ADDR1)
test_endpoint "Get address transactions" "$RESULT" '"transactions"'

echo ""

# ============================================================
# SUMMARY
# ============================================================
echo "============================================================"
echo -e "📊 ${YELLOW}TEST SUMMARY${NC}"
echo "============================================================"
echo ""
echo -e "   ${GREEN}PASSED: $PASS${NC}"
echo -e "   ${RED}FAILED: $FAIL${NC}"
echo ""

TOTAL=$((PASS + FAIL))
if [ $FAIL -eq 0 ]; then
    echo -e "   ${GREEN}✅ ALL $TOTAL TESTS PASSED!${NC}"
else
    echo -e "   ${RED}❌ $FAIL/$TOTAL TESTS FAILED${NC}"
fi

echo ""
echo "============================================================"
echo "📦 CONTRACTS DEPLOYED"
echo "============================================================"
echo "   Counter:   $COUNTER"
echo "   Whitelist: $WHITELIST"  
echo "   Vault:     $VAULT"
echo "   Token:     $TOKEN"
echo ""
echo "============================================================"
echo "🔑 WALLETS"
echo "============================================================"
echo "   Wallet 1: $ADDR1"
echo "   Wallet 2: $ADDR2"
echo ""
echo "============================================================"
echo "📡 FREE READ ENDPOINTS (No signature needed)"
echo "============================================================"
echo "   GET /contract/:addr/var/:name"
echo "   GET /contract/:addr/mapping/:name/:key"
echo "   GET /contract/:addr/call/get_*"
echo "   GET /contract/:addr/mbi"
echo ""