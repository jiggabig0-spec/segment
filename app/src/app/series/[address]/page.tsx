import { SeriesView } from "@/components/SeriesView";

export default async function SeriesPage({ params }: { params: Promise<{ address: string }> }) {
  const { address } = await params;
  return <SeriesView address={address} />;
}
