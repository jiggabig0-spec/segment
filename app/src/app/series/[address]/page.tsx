import { SeriesView } from "@/components/SeriesView";
import { DEMO_SERIES } from "@/lib/demo";

// Static export (used for the demo site) needs every page listed up front; the live site
// renders any series address on demand.
export function generateStaticParams() {
  return process.env.STATIC_EXPORT ? [{ address: DEMO_SERIES.address.toBase58() }] : [];
}

export default async function SeriesPage({ params }: { params: Promise<{ address: string }> }) {
  const { address } = await params;
  return <SeriesView address={address} />;
}
