#!/bin/bash

echo "🧠 Setting up The Brain development environment..."

# Start Supabase if not running
echo "Starting Supabase..."
supabase start

# Ensure base data exists without clearing
echo "Ensuring base data exists..."
node scripts/ensure-base-data.js

echo "✅ Development environment ready!"
echo "🚀 Starting Next.js development server..."

# Start the dev server
npm run dev