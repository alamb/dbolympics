# Analytics handbook

This handbook explains how the analytics team at Tidewick turns questions into numbers.

## The people

- **Leaders** (CEO, CFO, department heads) bring broad questions about their area.
- **Analysts** each work for one leader. An analyst:
  - turns the leader's questions into concrete report requests;
  - reads the printed reports and follows up;
  - writes a short memo for the leader at the end of the week.
- **Data engineers** turn each report request into one SQL query. They can check the query against the schema as often as they like before submitting it.

## The warehouse is expensive

The warehouse is shared and metered, and every executed query is billed to the department. We therefore run reports as an overnight batch:

- An analyst has **3 report slots per night** and the week has **4 nights**.
- The memo is due on Friday morning.
- A slot is spent whenever a request is submitted, whether it runs, fails or is bounced by review.
- Unused slots do not carry over.
- Checking a query against the schema (a "dry run") is free. It runs the query against an empty copy of the warehouse schema, so it catches errors and shows the output columns, but it never reads or shows any data.

Think before you request. Make one request answer one question well.

## One request, one report

Each request produces one printed report:

- **Title and question.** Every report has a title and the analyst's question it answers.
- **One grain.** A report is one table at a single grain: every row is the same kind of thing (a country, a week, a merchant, a carrier and month). The columns are measures or attributes of that thing.
- **The page has a width.** A report has at most 12 columns, and about 10 is comfortable.
- **The page has a length.** At most 50 rows are printed. The report notes how many rows the query returned in total.
- **Readable columns.** Order the rows so the important ones come first, and give columns readable names.

Dense reports are good. For example, one row per country and month can carry these columns:

- orders;
- revenue;
- refund rate;
- share of late deliveries;
- tickets per 100 orders;
- the same month last year.

Data engineering reviews every request before it runs and **bounces bundled ones**, where several unrelated questions are packed into one report. Reports wider than 12 columns are bounced too. A bounced request still uses its slot.

If you have two questions, request two reports.

## House SQL standards (for data engineers)

- Write **one SELECT statement**. Do not end it with a semicolon.
- Use **standard SQL** wherever possible.
- **Dates:** never use `CURRENT_DATE` or `now()`. The reporting date is 2026-01-05 and the data ends on 2025-12-31.
- **Deterministic ordering:** always finish with an ORDER BY whose tie-breaks make the order unique.
- **Tenants:** filter or group by `tenant_id` when a question is about particular merchants. Join `tenants` for merchant names.
- **Money:** amounts are in the currency given next to them (usually a `currency_code` column; a merchant's main currency is `tenants.default_currency_code`).
  - Convert amounts before adding them up across merchants that use different currencies.
  - `exchange_rates` holds a rate from EUR (`base_code`) for every day; weekends carry the previous business day's rate.
- **Rates:** round rates and averages for the page.
- **Catalog:** the table catalog (`schema.md`) lists every table, column, foreign key and the allowed codes of status columns.
