import type { Metadata } from 'next'
import './globals.css'

export const metadata: Metadata = {
  title: 'The Brain',
  description: 'A 3D visualization system for knowledge management and idea exploration',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <body className="h-screen w-screen overflow-hidden">{children}</body>
    </html>
  )
}