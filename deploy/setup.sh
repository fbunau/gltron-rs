#!/usr/bin/env bash
# One-time setup for Firebase Hosting deployment
set -e

echo "=== Firebase Hosting Setup ==="
echo ""

# Check firebase CLI
if ! command -v firebase &> /dev/null; then
    echo "Firebase CLI not found. Installing..."
    npm install -g firebase-tools
fi

echo "Logging into Firebase..."
firebase login

echo ""
echo "Available Firebase projects:"
firebase projects:list

echo ""
echo "Next steps:"
echo "  1. Edit .firebaserc and replace YOUR_FIREBASE_PROJECT_ID with your project ID"
echo "     (or create a new project at https://console.firebase.google.com)"
echo "  2. Run: ./deploy/deploy.sh"
echo ""
