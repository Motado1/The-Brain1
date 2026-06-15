#!/bin/bash

# Start The Brain with local RAG system
echo "🧠 Starting The Brain - Local RAG System"

# Check if Supabase is running
if ! curl -s http://localhost:54321/health > /dev/null; then
    echo "📦 Starting Supabase..."
    npx supabase start
fi

# Check if Functions are running
if ! curl -s http://localhost:54321/functions/v1/ > /dev/null; then
    echo "⚡ Starting Edge Functions..."
    npx supabase functions serve --env-file .env.local &
    sleep 5
fi

# Check if Next.js is running
if ! curl -s http://localhost:3001 > /dev/null; then
    echo "🌐 Starting Next.js app..."
    npm run dev -- --port 3001 &
    sleep 5
fi

# Start the local cron simulation
echo "⏰ Starting RAG job processor (simulated cron)..."
./simulate-cron.sh &

echo ""
echo "🎉 The Brain is ready!"
echo "   App: http://localhost:3001"
echo "   Database: http://localhost:54321"
echo "   Functions: http://localhost:54321/functions/v1/"
echo ""
echo "🤖 RAG System Status:"
echo "   - Job processor running every minute"
echo "   - File uploads to local storage"
echo "   - Mock embeddings (no external AI needed)"
echo "   - All data stored locally"
echo ""
echo "Press Ctrl+C to stop all services"

# Wait for interrupt
wait