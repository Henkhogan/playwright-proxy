#!/bin/bash
# Package Helm Chart Script
# This script packages the Helm chart locally

set -e

echo "📦 Packaging Playwright Proxy Helm Chart..."
echo ""

CHART_DIR="./helm/playwright-proxy"
OUTPUT_DIR="./helm-packages"

if [ ! -d "$CHART_DIR" ]; then
    echo "❌ Chart directory not found: $CHART_DIR"
    exit 1
fi

# Check if helm is installed
if ! command -v helm &> /dev/null; then
    echo "❌ Helm is not installed. Please install Helm to continue."
    exit 1
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Package the chart
echo "📦 Packaging chart..."
if helm package "$CHART_DIR" -d "$OUTPUT_DIR"; then
    echo "✅ Chart packaged successfully"
else
    echo "❌ Chart packaging failed"
    exit 1
fi
echo ""

# List packaged files
echo "📁 Packaged files in $OUTPUT_DIR:"
ls -lh "$OUTPUT_DIR"
echo ""

# Generate index (optional)
if [ "$1" = "--index" ]; then
    echo "📋 Generating Helm repository index..."
    if helm repo index "$OUTPUT_DIR"; then
        echo "✅ Repository index generated"
    else
        echo "❌ Repository index generation failed"
        exit 1
    fi
    echo ""
fi

echo "✨ Packaging complete!"
echo ""
echo "📝 To use the packaged chart:"
echo "   helm install my-release $OUTPUT_DIR/*.tgz"
