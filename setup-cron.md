# Setting Up Cron Schedule for Ingestion Worker

## Local Development

For local development, you can simulate the cron behavior by running the worker periodically:

### Option 1: Manual Testing
```bash
# Test the worker manually
curl -X POST "http://localhost:54321/functions/v1/ingestion-worker" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6ImFub24iLCJleHAiOjE5ODM4MTI5OTZ9.CRXP1A7WOeoJeXxjNni43kdQwgnWNReilDMblYTn_I0"
```

### Option 2: Local Cron Simulation
```bash
# Create a script to run every minute
echo '#!/bin/bash
curl -X POST "http://localhost:54321/functions/v1/ingestion-worker" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6ImFub24iLCJleHAiOjE5ODM4MTI5OTZ9.CRXP1A7WOeoJeXxjNni43kdQwgnWNReilDMblYTn_I0" \
  -s > /dev/null
' > run-ingestion-worker.sh

chmod +x run-ingestion-worker.sh

# Add to crontab (runs every minute)
echo "* * * * * $(pwd)/run-ingestion-worker.sh" | crontab -
```

## Production Deployment

### Step 1: Deploy the Function
```bash
npx supabase functions deploy ingestion-worker
```

### Step 2: Set Up Cron in Supabase Dashboard

1. **Go to Supabase Dashboard**
   - Navigate to your project
   - Go to Edge Functions section
   - Select "Cron Jobs"

2. **Create New Cron Job**
   - Click "Create a new cron job"
   - Set the following:
     - **Name**: `ingestion-worker-cron`
     - **Function**: `ingestion-worker`
     - **Schedule**: `* * * * *` (every minute)
     - **Description**: "Process RAG ingestion jobs every minute"

3. **Alternative: Using Supabase CLI**
```bash
# Create cron job via CLI (when available)
npx supabase functions cron create \
  --name ingestion-worker-cron \
  --schedule "* * * * *" \
  --function ingestion-worker
```

## Cron Schedule Options

- `* * * * *` - Every minute (recommended for active processing)
- `*/5 * * * *` - Every 5 minutes (for moderate load)
- `0 * * * *` - Every hour (for batch processing)
- `0 */6 * * *` - Every 6 hours (for low-frequency processing)

## Monitoring

Monitor the cron job execution:
1. Check Supabase Dashboard → Edge Functions → Logs
2. Monitor job_queue table for processing status
3. Check artifacts table for completion rates

## Environment Variables

Ensure these are set in Supabase Dashboard:
- `SUPABASE_URL`
- `SUPABASE_SERVICE_ROLE_KEY`
- `OLLAMA_URL` (optional, for production Ollama)
- `QDRANT_URL` (optional, for production Qdrant)
- `DEVELOPMENT_MODE=false` (for production)

## Testing the Cron

After setup, test by:
1. Creating a new artifact via API
2. Waiting 1-2 minutes for cron to trigger
3. Checking artifact status changes to 'indexed'
4. Verifying job_queue shows completed jobs