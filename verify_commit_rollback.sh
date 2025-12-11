#!/bin/bash

echo "=== Manual Verification Tests ==="
echo ""

# Test 1: Begin and Commit
echo "Test 1: Full Transaction Lifecycle (Begin -> Commit)"
TXN_RESULT=$(curl -s -X POST http://localhost:8080/graphql -H "Content-Type: application/json" -d '{"query":"mutation { beginTransaction(isolationLevel: SERIALIZABLE) { transactionId status } }"}')
TXN_ID=$(echo "$TXN_RESULT" | jq -r '.data.beginTransaction.transactionId')
echo "Created Transaction: $TXN_ID"

COMMIT_RESULT=$(curl -s -X POST http://localhost:8080/graphql -H "Content-Type: application/json" -d "{\"query\":\"mutation { commitTransaction(transactionId: \\\"$TXN_ID\\\") { transactionId status timestamp } }\"}")
echo "Commit Result:"
echo "$COMMIT_RESULT" | jq '.'
echo ""

# Test 2: Begin and Rollback
echo "Test 2: Full Transaction Lifecycle (Begin -> Rollback)"
TXN_RESULT=$(curl -s -X POST http://localhost:8080/graphql -H "Content-Type: application/json" -d '{"query":"mutation { beginTransaction(isolationLevel: READ_COMMITTED) { transactionId status } }"}')
TXN_ID=$(echo "$TXN_RESULT" | jq -r '.data.beginTransaction.transactionId')
echo "Created Transaction: $TXN_ID"

ROLLBACK_RESULT=$(curl -s -X POST http://localhost:8080/graphql -H "Content-Type: application/json" -d "{\"query\":\"mutation { rollbackTransaction(transactionId: \\\"$TXN_ID\\\") { transactionId status timestamp } }\"}")
echo "Rollback Result:"
echo "$ROLLBACK_RESULT" | jq '.'
echo ""

# Test 3: Invalid Transaction ID
echo "Test 3: Commit with Invalid Transaction ID"
INVALID_RESULT=$(curl -s -X POST http://localhost:8080/graphql -H "Content-Type: application/json" -d '{"query":"mutation { commitTransaction(transactionId: \"invalid-txn-id-12345\") { transactionId status } }"}')
echo "Result:"
echo "$INVALID_RESULT" | jq '.'
echo ""

# Test 4: Double Commit
echo "Test 4: Double Commit Test"
TXN_RESULT=$(curl -s -X POST http://localhost:8080/graphql -H "Content-Type: application/json" -d '{"query":"mutation { beginTransaction { transactionId status } }"}')
TXN_ID=$(echo "$TXN_RESULT" | jq -r '.data.beginTransaction.transactionId')
echo "Created Transaction: $TXN_ID"

COMMIT1=$(curl -s -X POST http://localhost:8080/graphql -H "Content-Type: application/json" -d "{\"query\":\"mutation { commitTransaction(transactionId: \\\"$TXN_ID\\\") { status } }\"}")
echo "First Commit: $(echo "$COMMIT1" | jq -r '.data.commitTransaction.status')"

COMMIT2=$(curl -s -X POST http://localhost:8080/graphql -H "Content-Type: application/json" -d "{\"query\":\"mutation { commitTransaction(transactionId: \\\"$TXN_ID\\\") { status } }\"}")
echo "Second Commit Attempt:"
echo "$COMMIT2" | jq '.'
echo ""

# Test 5: Concurrent Transactions
echo "Test 5: Concurrent Transactions with Commit"
echo "Creating 3 concurrent transactions..."
TXN1=$(curl -s -X POST http://localhost:8080/graphql -H "Content-Type: application/json" -d '{"query":"mutation { beginTransaction(isolationLevel: SERIALIZABLE) { transactionId } }"}' | jq -r '.data.beginTransaction.transactionId')
TXN2=$(curl -s -X POST http://localhost:8080/graphql -H "Content-Type: application/json" -d '{"query":"mutation { beginTransaction(isolationLevel: READ_COMMITTED) { transactionId } }"}' | jq -r '.data.beginTransaction.transactionId')
TXN3=$(curl -s -X POST http://localhost:8080/graphql -H "Content-Type: application/json" -d '{"query":"mutation { beginTransaction(isolationLevel: REPEATABLE_READ) { transactionId } }"}' | jq -r '.data.beginTransaction.transactionId')

echo "TXN1: $TXN1"
echo "TXN2: $TXN2"
echo "TXN3: $TXN3"

echo "Committing all transactions..."
RESULT1=$(curl -s -X POST http://localhost:8080/graphql -H "Content-Type: application/json" -d "{\"query\":\"mutation { commitTransaction(transactionId: \\\"$TXN1\\\") { status } }\"}" | jq -r '.data.commitTransaction.status')
RESULT2=$(curl -s -X POST http://localhost:8080/graphql -H "Content-Type: application/json" -d "{\"query\":\"mutation { commitTransaction(transactionId: \\\"$TXN2\\\") { status } }\"}" | jq -r '.data.commitTransaction.status')
RESULT3=$(curl -s -X POST http://localhost:8080/graphql -H "Content-Type: application/json" -d "{\"query\":\"mutation { commitTransaction(transactionId: \\\"$TXN3\\\") { status } }\"}" | jq -r '.data.commitTransaction.status')

echo "Results: $RESULT1, $RESULT2, $RESULT3"

if [ "$RESULT1" = "COMMITTED" ] && [ "$RESULT2" = "COMMITTED" ] && [ "$RESULT3" = "COMMITTED" ]; then
    echo "✅ All 3 concurrent transactions committed successfully"
else
    echo "❌ Some transactions failed to commit"
fi
echo ""

echo "=== Verification Complete ==="
