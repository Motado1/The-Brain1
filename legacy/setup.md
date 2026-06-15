# The Brain Setup Guide

## 1. Supabase Setup

### Create Supabase Project
1. Go to https://supabase.com
2. Sign up or log in
3. Click "New Project"
4. Choose your organization
5. Enter project name: "the-brain"
6. Generate a strong password
7. Select region (closest to you)
8. Click "Create new project"

### Configure Database
1. Wait for project to finish setting up
2. Go to SQL Editor in your Supabase dashboard
3. Copy and paste the content from `database/schema.sql`
4. Click "Run" to create tables and policies
5. Copy and paste the content from `database/seed.sql`
6. Click "Run" to insert sample data

### Get API Keys
1. Go to Settings → API in your Supabase dashboard
2. Copy the Project URL
3. Copy the anon public key
4. Update `.env.local` with these values:

```env
NEXT_PUBLIC_SUPABASE_URL=YOUR_PROJECT_URL_HERE
NEXT_PUBLIC_SUPABASE_ANON_KEY=YOUR_ANON_KEY_HERE
```

## 2. Run the Application

```bash
npm run dev
```

The application will be available at http://localhost:3000

## 3. Verify Setup

You should see:
- A black 3D canvas taking most of the screen
- 6 colored spheres representing the pillar nodes
- A gray info panel on the right
- Ability to click nodes to select them
- Stats panel in the top-left corner
- Mouse controls to rotate/zoom the 3D view

## Troubleshooting

If nodes don't appear:
1. Check browser console for errors
2. Verify your Supabase URL and key are correct
3. Check that the SQL scripts ran successfully
4. Verify the tables contain data in Supabase Table Editor