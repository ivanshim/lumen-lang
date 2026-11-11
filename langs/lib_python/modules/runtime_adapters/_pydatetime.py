# Expose native read-only members over the fallback's original slot storage.
timedelta.days = __readonly_member__('days', timedelta._days)
timedelta.seconds = __readonly_member__('seconds', timedelta._seconds)
timedelta.microseconds = __readonly_member__('microseconds', timedelta._microseconds)
