#!/bin/bash

# Simulate cron behavior for ingestion worker
# Runs the worker every minute for testing

echo "🕐 Starting ingestion worker cron simulation..."
echo "Press Ctrl+C to stop"

FUNCTION_URL="http://localhost:54321/functions/v1/ingestion-worker"
ANON_KEY="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6ImFub24iLCJleHAiOjE5ODM4MTI5OTZ9.CRXP1A7WOeoJeXxjNni43kdQwgnWNReilDMblYTn_I0"

while true; do
    TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$TIMESTAMP] 🔄 Triggering ingestion worker..."
    
    RESPONSE=$(curl -s -X POST "$FUNCTION_URL" \
        -H "Authorization: Bearer $ANON_KEY" \
        -H "Content-Type: application/json" \
        -d '{}')
    
    # Parse response and show summary
    if echo "$RESPONSE" | grep -q "Job processed successfully"; then
        JOB_ID=$(echo "$RESPONSE" | grep -o '"jobId":"[^"]*"' | cut -d'"' -f4)
        echo "[$TIMESTAMP] ✅ Processed job: $JOB_ID"
    elif echo "$RESPONSE" | grep -q "No jobs available"; then
        echo "[$TIMESTAMP] 💤 No jobs to process"
    elif echo "$RESPONSE" | grep -q "error"; then
        ERROR=$(echo "$RESPONSE" | grep -o '"message":"[^"]*"' | cut -d'"' -f4)
        echo "[$TIMESTAMP] ❌ Error: $ERROR"
    else
        echo "[$TIMESTAMP] ❓ Unknown response: $RESPONSE"
    fi
    
    # Wait 60 seconds (1 minute)
    sleep 60
done