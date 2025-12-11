#!/bin/bash

# ML-001: Test GraphQL introspection for ML types
echo "ML-001: GraphQL Schema Introspection"
curl -s -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{ __type(name: \"Query\") { fields { name description } } }"
  }' | python3 -m json.tool 2>/dev/null || echo "RESPONSE: $(curl -s -X POST http://localhost:8080/graphql -H 'Content-Type: application/json' -d '{\"query\": \"{ __type(name: \\\"Query\\\") { fields { name description } } }\"}')"
