export function prepareTimelineActivities(activitiesData) {
  return [...activitiesData].sort((a, b) => {
    if (b.timestamp !== a.timestamp) {
      return b.timestamp - a.timestamp;
    }
    return (b.id || 0) - (a.id || 0);
  });
}

export function upsertTimelineActivity(currentActivities, newActivity) {
  const existingById = currentActivities.findIndex((activity) => activity.id === newActivity.id);
  if (existingById >= 0) {
    return currentActivities.map((activity) =>
      activity.id === newActivity.id ? newActivity : activity
    );
  }

  return prepareTimelineActivities([newActivity, ...currentActivities]);
}
