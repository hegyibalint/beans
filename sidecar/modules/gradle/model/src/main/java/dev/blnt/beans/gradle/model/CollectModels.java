package dev.blnt.beans.gradle.model;

import java.util.ArrayList;
import java.util.List;
import org.gradle.tooling.BuildAction;
import org.gradle.tooling.BuildController;
import org.gradle.tooling.model.gradle.BasicGradleProject;

public final class CollectModels implements BuildAction<List<ProjectModel>> {
    private static final long serialVersionUID = 1L;

    @Override
    public List<ProjectModel> execute(BuildController controller) {
        List<ProjectModel> projects = new ArrayList<>();
        // Import the requested build, not every included build providing its build logic.
        for (BasicGradleProject project : controller.getBuildModel().getProjects()) {
            // Copy adapted views so the result contains only our serializable data.
            projects.add(new Snapshots.Project(controller.getModel(project, ProjectModel.class)));
        }
        return projects;
    }
}
