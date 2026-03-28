use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone};
use chrono_tz::Asia::Shanghai;
use mysql::prelude::Queryable;
use mysql::{Pool, PooledConn, Row};
use serde::{Deserialize, Serialize};
use spring_boot::web::{HttpRequest, HttpResponse};
use spring_boot::{Application, ApplicationContext, Component, HttpServer, PostMapping};

// ─────────────────────────────────────────────────────────────────────────────
// Request/Response models (match Java contract)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ActivityQueryRequest {
    page: Option<i32>,
    page_size: Option<i32>,

    #[serde(default)]
    r#type: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    functionary: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    start_from: Option<String>,
    #[serde(default)]
    start_to: Option<String>,
    #[serde(default)]
    is_full: Option<bool>,

    #[serde(default)]
    cursor_start_time: Option<String>,
    #[serde(default)]
    cursor_id: Option<String>,

    // Present in Java VO but not used
    #[serde(default)]
    #[allow(dead_code)]
    sort_by: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    sort_order: Option<String>,
}

#[derive(Debug, Serialize)]
struct ResultEnvelope<T> {
    code: i32,
    message: &'static str,
    data: T,
}

impl<T> ResultEnvelope<T> {
    fn success(data: T) -> Self {
        Self {
            code: 200,
            message: "success",
            data,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivityPageResponse {
    items: Vec<ActivityItemResponse>,
    total: i64,
    page: i32,
    page_size: i32,
    has_more: bool,
    next_cursor_start_time: Option<String>,
    next_cursor_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivityItemResponse {
    id: String,
    functionary: Option<String>,
    name: Option<String>,
    r#type: Option<String>,
    description: Option<String>,

    #[serde(rename = "EnrollmentStartTime")]
    enrollment_start_time: Option<String>,
    #[serde(rename = "EnrollmentEndTime")]
    enrollment_end_time: Option<String>,

    start_time: Option<String>,
    expected_end_time: Option<String>,
    end_time: Option<String>,

    #[serde(rename = "CoverPath")]
    cover_path: Option<String>,
    #[serde(rename = "CoverImage")]
    cover_image: Option<String>,

    #[serde(rename = "maxParticipants")]
    max_participants: Option<i32>,

    #[serde(rename = "Attachment")]
    attachment: Option<Vec<String>>,

    participants: Option<Vec<String>>,
    status: Option<String>,
    is_full: Option<bool>,
    duration: Option<f64>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Principal emulation from headers
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct Principal {
    role: Option<String>,
    student_no: Option<String>,
}

fn principal_from_headers(req: &HttpRequest) -> Principal {
    Principal {
        role: req.header("x-role").map(|s| s.to_string()),
        student_no: req.header("x-student-no").map(|s| s.to_string()),
    }
}

fn is_admin(p: &Principal) -> bool {
    matches!(
        p.role.as_deref(),
        Some("admin") | Some("superAdmin") | Some("ADMIN") | Some("SUPERADMIN")
    )
}

// Mirrors Java rule:
// useAll = status != null || isAdmin || (role==functionary && functionary == studentNo)
// ─────────────────────────────────────────────────────────────────────────────
// Time helpers (match Java: parse OffsetDateTime, convert to Asia/Shanghai LocalDateTime)
// ─────────────────────────────────────────────────────────────────────────────

fn parse_offset_datetime_to_shanghai_naive(s: &str) -> Result<NaiveDateTime, String> {
    let dt: DateTime<FixedOffset> =
        DateTime::parse_from_rfc3339(s).map_err(|e| format!("invalid datetime '{}': {}", s, e))?;
    let sh = dt.with_timezone(&Shanghai);
    Ok(sh.naive_local())
}

fn naive_to_rfc3339_shanghai(naive: NaiveDateTime) -> String {
    let dt = Shanghai
        .from_local_datetime(&naive)
        .single()
        .unwrap_or_else(|| {
            // fallback: pick earliest if ambiguous
            Shanghai.from_local_datetime(&naive).earliest().unwrap()
        });
    dt.to_rfc3339()
}

// ─────────────────────────────────────────────────────────────────────────────
// DB layer
// ─────────────────────────────────────────────────────────────────────────────

#[Component]
#[derive(Debug, Clone, Default)]
struct DbConfig {
    #[Value("${volunteer.db.url:}")]
    db_url: String,

    #[Value("${server.port:8081}")]
    port: i32,
}

#[Component]
#[derive(Debug, Default, Clone)]
struct ActivityRepository {
    #[autowired]
    cfg: DbConfig,
    pool: std::sync::Arc<std::sync::OnceLock<Pool>>,
}

impl ActivityRepository {
    fn pool(&self) -> Result<&Pool, String> {
        if self.cfg.db_url.trim().is_empty() {
            return Err(
                "volunteer.db.url is required in application.properties (e.g. mysql://user:pass@host:3306/db)"
                    .to_string(),
            );
        }

        let db_url = self.cfg.db_url.clone();
        Ok(self.pool.get_or_init(|| {
            let opts = mysql::Opts::from_url(&db_url)
                .unwrap_or_else(|e| panic!("parse volunteer.db.url: {}", e));
            Pool::new(opts).unwrap_or_else(|e| panic!("create mysql pool: {}", e))
        }))
    }
}

impl ActivityRepository {
    fn conn(&self) -> Result<PooledConn, String> {
        self.pool()?.get_conn().map_err(|e| e.to_string())
    }

    fn count_filtered(&self, q: &NormalizedQuery, exclude_hidden: bool) -> Result<i64, String> {
        let mut conn = self.conn()?;

        let mut sql = String::from("SELECT COUNT(1) as cnt FROM activities");
        let (where_sql, params_map) = build_where_clause(q, exclude_hidden, false);
        if !where_sql.is_empty() {
            sql.push(' ');
            sql.push_str(&where_sql);
        }

        let row: Option<Row> = conn
            .exec_first(sql, mysql::Params::Named(params_map))
            .map_err(|e| e.to_string())?;
        let cnt: i64 = row
            .and_then(|r| mysql::from_row_opt::<(i64,)>(r).ok())
            .map(|t| t.0)
            .unwrap_or(0);
        Ok(cnt)
    }

    fn list_paged(
        &self,
        q: &NormalizedQuery,
        exclude_hidden: bool,
    ) -> Result<Vec<ActivityRow>, String> {
        let mut conn = self.conn()?;

        let mut sql = String::from(
            "SELECT id, functionary, name, type, description, \
                    enrollment_start_time, enrollment_end_time, \
                    start_time, expected_end_time, end_time, cover_path, max_participants, status, is_full, duration \
             FROM activities",
        );

        let (where_sql, mut params_map) = build_where_clause(q, exclude_hidden, false);
        if !where_sql.is_empty() {
            sql.push(' ');
            sql.push_str(&where_sql);
        }

        sql.push_str(" ORDER BY start_time DESC LIMIT :limit OFFSET :offset");
        params_map.insert(b"limit".to_vec(), q.limit.into());
        params_map.insert(b"offset".to_vec(), q.offset.into());

        let rows: Vec<Row> = conn
            .exec(sql, mysql::Params::Named(params_map))
            .map_err(|e| e.to_string())?;
        rows.into_iter().map(ActivityRow::try_from_row).collect()
    }

    fn list_by_cursor(
        &self,
        q: &NormalizedQuery,
        exclude_hidden: bool,
    ) -> Result<Vec<ActivityRow>, String> {
        let mut conn = self.conn()?;

        let mut sql = String::from(
            "SELECT id, functionary, name, type, description, \
                    enrollment_start_time, enrollment_end_time, \
                    start_time, expected_end_time, end_time, cover_path, max_participants, status, is_full, duration \
             FROM activities",
        );

        let (where_sql, mut params_map) = build_where_clause(q, exclude_hidden, true);
        if !where_sql.is_empty() {
            sql.push(' ');
            sql.push_str(&where_sql);
        }

        sql.push_str(" ORDER BY start_time DESC, id DESC LIMIT :limit");
        params_map.insert(b"limit".to_vec(), q.limit.into());

        let rows: Vec<Row> = conn
            .exec(sql, mysql::Params::Named(params_map))
            .map_err(|e| e.to_string())?;
        rows.into_iter().map(ActivityRow::try_from_row).collect()
    }
}

#[derive(Debug, Clone)]
struct NormalizedQuery {
    page: i32,
    page_size: i32,
    offset: i32,
    limit: i32,

    r#type: Option<String>,
    status: Option<String>,
    functionary: Option<String>,
    name: Option<String>,
    start_from: Option<NaiveDateTime>,
    start_to: Option<NaiveDateTime>,
    is_full: Option<bool>,

    cursor_start_time: Option<NaiveDateTime>,
    cursor_id: Option<String>,
    use_cursor: bool,
}

#[derive(Debug, Clone)]
struct ActivityRow {
    id: String,
    functionary: Option<String>,
    name: Option<String>,
    r#type: Option<String>,
    description: Option<String>,

    enrollment_start_time: Option<NaiveDateTime>,
    enrollment_end_time: Option<NaiveDateTime>,

    start_time: Option<NaiveDateTime>,
    expected_end_time: Option<NaiveDateTime>,
    end_time: Option<NaiveDateTime>,

    cover_path: Option<String>,
    max_participants: Option<i32>,
    status: Option<String>,
    is_full: Option<bool>,
    duration: Option<f64>,
}

impl ActivityRow {
    fn try_from_row(mut r: Row) -> Result<Self, String> {
        Ok(Self {
            id: r.take("id").ok_or_else(|| "missing id".to_string())?,
            functionary: r.take("functionary"),
            name: r.take("name"),
            r#type: r.take("type"),
            description: r.take("description"),
            enrollment_start_time: r.take("enrollment_start_time"),
            enrollment_end_time: r.take("enrollment_end_time"),
            start_time: r.take("start_time"),
            expected_end_time: r.take("expected_end_time"),
            end_time: r.take("end_time"),
            cover_path: r.take("cover_path"),
            max_participants: r.take("max_participants"),
            status: r.take("status"),
            is_full: r.take("is_full"),
            duration: r.take("duration"),
        })
    }
}

fn build_where_clause(
    q: &NormalizedQuery,
    exclude_hidden: bool,
    include_cursor_predicate: bool,
) -> (String, std::collections::HashMap<Vec<u8>, mysql::Value>) {
    // We build MyBatis-like dynamic conditions with named params.
    let mut clauses: Vec<String> = Vec::new();
    let mut params_map: std::collections::HashMap<Vec<u8>, mysql::Value> =
        std::collections::HashMap::new();

    let mut insert_param = |k: &'static str, v: mysql::Value| {
        params_map.insert(k.as_bytes().to_vec(), v);
    };

    if let Some(t) = q.r#type.as_deref() {
        clauses.push("type = :type".to_string());
        insert_param("type", t.into());
    }

    if let Some(st) = q.status.as_deref() {
        clauses.push("status = :status".to_string());
        insert_param("status", st.into());
    } else if exclude_hidden {
        clauses.push("status NOT IN ('UnderReview','FailReview','ActivityEnded')".to_string());
    }

    if let Some(f) = q.functionary.as_deref() {
        if !f.is_empty() {
            clauses.push("functionary = :functionary".to_string());
            insert_param("functionary", f.into());
        }
    }

    if let Some(n) = q.name.as_deref() {
        if !n.is_empty() {
            clauses.push("name LIKE CONCAT('%', :name, '%')".to_string());
            insert_param("name", n.into());
        }
    }

    if let Some(sf) = q.start_from {
        clauses.push("start_time >= :startFrom".to_string());
        insert_param("startFrom", sf.into());
    }

    if let Some(st) = q.start_to {
        clauses.push("start_time <= :startTo".to_string());
        insert_param("startTo", st.into());
    }

    if let Some(full) = q.is_full {
        clauses.push("is_full = :isFull".to_string());
        insert_param("isFull", full.into());
    }

    if include_cursor_predicate {
        if let (Some(cursor_time), Some(cursor_id)) = (q.cursor_start_time, q.cursor_id.as_deref())
        {
            if !cursor_id.is_empty() {
                clauses.push("(start_time < :cursorStartTime OR (start_time = :cursorStartTime AND id < :cursorId))".to_string());
                insert_param("cursorStartTime", cursor_time.into());
                insert_param("cursorId", cursor_id.into());
            }
        }
    }

    if clauses.is_empty() {
        (String::new(), params_map)
    } else {
        (format!("WHERE {}", clauses.join(" AND ")), params_map)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Service layer
// ─────────────────────────────────────────────────────────────────────────────

#[Component]
#[derive(Debug, Clone, Default)]
struct ActivityQueryService {
    #[autowired]
    repo: ActivityRepository,
}

impl ActivityQueryService {
    fn query(
        &self,
        raw: ActivityQueryRequest,
        principal: Principal,
    ) -> Result<ActivityPageResponse, String> {
        let normalized = normalize_query(raw)?;

        let use_all = compute_use_all_raw(&normalized, &principal);
        let exclude_hidden = !use_all && normalized.status.is_none();

        let total = self.repo.count_filtered(&normalized, exclude_hidden)?;

        let mut rows = if normalized.use_cursor {
            self.repo.list_by_cursor(&normalized, exclude_hidden)?
        } else {
            self.repo.list_paged(&normalized, exclude_hidden)?
        };

        let page_size = normalized.page_size;
        let has_more = if normalized.use_cursor {
            rows.len() as i32 > page_size
        } else {
            // Keep parity with Java current behavior: offset mode never sets hasMore
            false
        };

        if normalized.use_cursor && has_more {
            rows.truncate(page_size as usize);
        }

        let (next_cursor_start_time, next_cursor_id) =
            if normalized.use_cursor && has_more && !rows.is_empty() {
                let last = rows.last().unwrap();
                let nct = last.start_time.map(naive_to_rfc3339_shanghai);
                (nct, Some(last.id.clone()))
            } else {
                (None, None)
            };

        let items = rows
            .into_iter()
            .map(ActivityItemResponse::from_row)
            .collect();

        Ok(ActivityPageResponse {
            items,
            total,
            page: normalized.page,
            page_size: normalized.page_size,
            has_more,
            next_cursor_start_time,
            next_cursor_id,
        })
    }
}

fn compute_use_all_raw(q: &NormalizedQuery, p: &Principal) -> bool {
    if q.status.is_some() {
        return true;
    }
    if is_admin(p) {
        return true;
    }
    if p.role.as_deref() == Some("functionary") {
        if let (Some(f), Some(sn)) = (q.functionary.as_deref(), p.student_no.as_deref()) {
            if f == sn {
                return true;
            }
        }
    }
    false
}

fn normalize_query(raw: ActivityQueryRequest) -> Result<NormalizedQuery, String> {
    let mut page = raw.page.unwrap_or(1);
    let mut page_size = raw.page_size.unwrap_or(10);
    if page < 1 {
        page = 1;
    }
    if page_size < 1 {
        page_size = 1;
    }
    if page_size > 100 {
        page_size = 100;
    }

    let start_from = match raw.start_from.as_deref() {
        Some(s) if !s.is_empty() => Some(parse_offset_datetime_to_shanghai_naive(s)?),
        _ => None,
    };
    let start_to = match raw.start_to.as_deref() {
        Some(s) if !s.is_empty() => Some(parse_offset_datetime_to_shanghai_naive(s)?),
        _ => None,
    };

    let cursor_start_time = match raw.cursor_start_time.as_deref() {
        Some(s) if !s.is_empty() => Some(parse_offset_datetime_to_shanghai_naive(s)?),
        _ => None,
    };
    let cursor_id = raw
        .cursor_id
        .and_then(|s| if s.trim().is_empty() { None } else { Some(s) });

    let use_cursor = cursor_start_time.is_some() && cursor_id.is_some();

    let limit = if use_cursor { page_size + 1 } else { page_size };
    let offset = ((page - 1) * page_size).max(0);

    Ok(NormalizedQuery {
        page,
        page_size,
        offset,
        limit,
        r#type: raw.r#type,
        status: raw.status,
        functionary: raw.functionary,
        name: raw.name,
        start_from,
        start_to,
        is_full: raw.is_full,
        cursor_start_time,
        cursor_id,
        use_cursor,
    })
}

impl ActivityItemResponse {
    fn from_row(r: ActivityRow) -> Self {
        // Note: coverImage handling in Java is enriched via file service.
        // For benchmark parity, we return coverImage as None unless you later add file service.
        Self {
            id: r.id,
            functionary: r.functionary,
            name: r.name,
            r#type: r.r#type,
            description: r.description,
            enrollment_start_time: r.enrollment_start_time.map(naive_to_rfc3339_shanghai),
            enrollment_end_time: r.enrollment_end_time.map(naive_to_rfc3339_shanghai),
            start_time: r.start_time.map(naive_to_rfc3339_shanghai),
            expected_end_time: r.expected_end_time.map(naive_to_rfc3339_shanghai),
            end_time: r.end_time.map(naive_to_rfc3339_shanghai),
            cover_path: r.cover_path,
            cover_image: None,
            max_participants: r.max_participants,
            attachment: None,
            participants: None,
            status: r.status,
            is_full: r.is_full,
            duration: r.duration,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Web handler
// ─────────────────────────────────────────────────────────────────────────────

#[PostMapping("/activities/query")]
fn query_activities(service: &ActivityQueryService, req: &HttpRequest) -> HttpResponse {
    if !req.is_json() {
        return HttpResponse::bad_request()
            .json(r#"{"code":400,"message":"Content-Type must be application/json","data":null}"#);
    }

    let principal = principal_from_headers(req);

    let parsed: ActivityQueryRequest = match serde_json::from_str(req.body_str()) {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::bad_request().json(
                serde_json::json!({
                    "code": 400,
                    "message": format!("invalid JSON body: {}", e),
                    "data": serde_json::Value::Null
                })
                .to_string(),
            )
        }
    };

    match service.query(parsed, principal) {
        Ok(page) => {
            HttpResponse::ok().json(serde_json::to_string(&ResultEnvelope::success(page)).unwrap())
        }
        Err(e) => HttpResponse::bad_request().json(
            serde_json::json!({
                "code": 400,
                "message": e,
                "data": serde_json::Value::Null
            })
            .to_string(),
        ),
    }
}

fn main() {
    println!("=== volunteer query demo ===");

    let context = Application::run();

    let port = context
        .get_bean("dbConfig")
        .and_then(|b| {
            b.as_ref()
                .downcast_ref::<DbConfig>()
                .and_then(|cfg| u16::try_from(cfg.port).ok())
        })
        .unwrap_or(8081);

    HttpServer::run_tokio(port, context);
}
