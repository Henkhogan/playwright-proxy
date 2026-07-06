#!/bin/bash
# Helm Chart Validation Script
# This script can be run locally to validate the Helm chart before committing

set -e

echo "🔍 Validating Playwright Proxy Helm Chart..."
echo ""

CHART_DIR="./helm/playwright-proxy"

if [ ! -d "$CHART_DIR" ]; then
    echo "❌ Chart directory not found: $CHART_DIR"
    exit 1
fi

# Check if helm is installed
if ! command -v helm &> /dev/null; then
    echo "❌ Helm is not installed. Please install Helm to continue."
    echo "   See: https://helm.sh/docs/intro/install/"
    exit 1
fi

echo "✅ Helm found: $(helm version --short)"
echo ""

# Lint the chart
echo "🔎 Linting chart..."
if helm lint "$CHART_DIR"; then
    echo "✅ Chart linting passed"
else
    echo "❌ Chart linting failed"
    exit 1
fi
echo ""

# Template the chart
echo "📋 Templating chart..."
if helm template playwright-proxy "$CHART_DIR" > /tmp/chart-template.yaml; then
    echo "✅ Chart templating passed"
else
    echo "❌ Chart templating failed"
    exit 1
fi
echo ""

# Check if values.yaml is valid YAML
echo "🔎 Validating values.yaml..."
if helm show values "$CHART_DIR" > /dev/null; then
    echo "✅ values.yaml is valid"
else
    echo "❌ values.yaml validation failed"
    exit 1
fi
echo ""

# Check Chart.yaml
echo "🔎 Validating Chart.yaml..."
if [ -f "$CHART_DIR/Chart.yaml" ]; then
    echo "✅ Chart.yaml found"
else
    echo "❌ Chart.yaml not found"
    exit 1
fi
echo ""

# List templates
echo "📁 Chart templates:"
ls -la "$CHART_DIR/templates/"
echo ""

# Show chart information
echo "📊 Chart information:"
helm show chart "$CHART_DIR"
echo ""

echo "✨ All validations passed!"
exit 0
