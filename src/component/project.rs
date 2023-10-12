use crate::context::GitlabContext;

#[derive(Debug, Clone)]
pub struct ProjectHandler {
    pub context: GitlabContext,
}

#[derive(Debug)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub full_path: String,
    pub web_url: String,
    pub topics: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct ProjectsWithPageInfo {
    pub projects: Vec<Project>,
    pub page_info: PageInfo,
}

#[derive(Debug)]
pub struct PageInfo {
    pub end_cursor: Option<String>,
    pub has_next_page: bool,
}

impl ProjectHandler {
    pub async fn fetch_group_projects(
        &self,
        group_full_path: &str,
        after_pointer_token: Option<String>,
    ) -> ProjectsWithPageInfo {
        let group_data = self
            .context
            .gitlab_graphql_client
            .fetch_group_projects(group_full_path, after_pointer_token)
            .await;
        // println!("group_data: {:?}", &group_data);

        let mut projects: Vec<Project> = Vec::new();
        for project in group_data
            .projects
            .nodes
            .expect("GroupProjectsNodes is None")
        {
            let project_ref = project.as_ref().expect("project is None");
            projects.push(Project {
                id: project_ref.id.clone(),
                name: project_ref.name.clone(),
                path: project_ref.path.clone(),
                full_path: project_ref.full_path.clone(),
                web_url: project_ref.web_url.clone(),
                topics: project_ref.topics.as_ref().cloned(),
            });
        }

        ProjectsWithPageInfo {
            projects,
            page_info: PageInfo {
                end_cursor: group_data.projects.page_info.end_cursor,
                has_next_page: group_data.projects.page_info.has_next_page,
            },
        }
    }

    pub async fn persist_project(&self, project: &Project) {
        let mut conn = self.context.store.conn_pool.acquire().await.unwrap();
        sqlx::query(
            r#"
            INSERT INTO engineering_metrics.projects (p_id, p_name, p_path, p_full_path, p_web_url, topics)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (p_id) DO 
            UPDATE SET 
                p_name = $2,
                p_path = $3,
                p_full_path = $4,
                p_web_url = $5,
                topics = $6
            "#)
            .bind(&project.id)
            .bind(&project.name)
            .bind(&project.path)
            .bind(&project.full_path)
            .bind(&project.web_url)
            .bind(serde_json::to_value(&project.topics).unwrap())
        .execute(&mut conn)
        .await
        .unwrap();
    }

    pub async fn import_projects(&self, group_full_path: &str) {
        let mut has_more = true;
        let mut after_pointer_token = Option::None;

        while has_more {
            let res = self
                .fetch_group_projects(group_full_path, after_pointer_token.to_owned())
                .await;

            for project in res.projects {
                self.persist_project(&project).await;
            }

            after_pointer_token = res.page_info.end_cursor;
            has_more = res.page_info.has_next_page;
        }
        println!(
            "Done importing projects data for group={}.",
            &group_full_path
        );
    }
}
