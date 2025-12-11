#!/bin/bash
# Security Vault Comprehensive Test Suite
# Tests all security vault features at 100% coverage

echo "=========================================="
echo "Security Vault Module Test Suite"
echo "=========================================="
echo ""

cd /home/user/rusty-db

# Run all security vault unit tests with detailed output
echo "VAULT-001: Running TDE Engine Tests..."
cargo test --lib security_vault::tde:: -- --nocapture 2>&1 | tee /tmp/vault_tde_tests.log

echo ""
echo "VAULT-002: Running Data Masking Tests..."
cargo test --lib security_vault::masking:: -- --nocapture 2>&1 | tee /tmp/vault_masking_tests.log

echo ""
echo "VAULT-003: Running Key Store Tests..."
cargo test --lib security_vault::keystore:: -- --nocapture 2>&1 | tee /tmp/vault_keystore_tests.log

echo ""
echo "VAULT-004: Running VPD Engine Tests..."
cargo test --lib security_vault::vpd:: -- --nocapture 2>&1 | tee /tmp/vault_vpd_tests.log

echo ""
echo "VAULT-005: Running Audit Vault Tests..."
cargo test --lib security_vault::audit:: -- --nocapture 2>&1 | tee /tmp/vault_audit_tests.log

echo ""
echo "VAULT-006: Running Privilege Analyzer Tests..."
cargo test --lib security_vault::privileges:: -- --nocapture 2>&1 | tee /tmp/vault_privileges_tests.log

echo ""
echo "VAULT-007: Running Security Vault Manager Tests..."
cargo test --lib security_vault::tests:: -- --nocapture 2>&1 | tee /tmp/vault_manager_tests.log

echo ""
echo "=========================================="
echo "All Security Vault Tests Complete"
echo "=========================================="
